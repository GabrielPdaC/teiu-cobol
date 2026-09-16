//! Motor do analisador léxico: conduz a troca entre os modos NORMAL e PIC e
//! traduz os tokens de `crate::token` para o tipo público [`Token`].
//! Dividido em três arquivos:
//!
//! - este arquivo — o laço que roda o `logos` ([`lex`]) e monta a lista de
//!   tokens, junto com os tipos públicos (`Token`, `TokenKind`, `LexError`).
//! - [`position`] — traduz a posição em bytes que o `logos` devolve para
//!   linha e coluna.
//! - [`diagnostics`] — as mensagens de erro léxico.

mod diagnostics;
mod position;

use logos::Logos;

use crate::token::{NormalToken, PicToken};

use diagnostics::{diagnose_invalid_word, to_text};
use position::LineIndex;

/// Tamanho máximo de um nome, em caracteres.
const MAX_NAME_LEN: usize = 31;

/// Um token já classificado, com a posição de onde começa na entrada.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Token {
    pub kind: TokenKind,
    pub line: usize,
    pub col: usize,
}

/// Tipo do token, com o lexema guardado como foi escrito quando ele varia
/// (nomes, números, cadeias PIC e as reservadas fora do escopo).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TokenKind {
    Data,
    Division,
    WorkingStorage,
    Section,
    Pic,
    Is,
    Redefines,
    UnsupportedReserved(String),
    Number(String),
    Name(String),
    Period,
    PicString(String),
}

/// Um erro léxico, sempre com linha e coluna.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LexError {
    pub message: String,
    pub line: usize,
    pub col: usize,
}

/// Analisa `source` e devolve os tokens reconhecidos e os erros léxicos
/// encontrados. Continua após cada erro, para relatar todos de uma vez
/// (recuperação simples: a regra pega-tudo de `crate::token` já consome a
/// palavra inválida inteira, então a próxima iteração começa limpa).
pub fn lex(source: &[u8]) -> (Vec<Token>, Vec<LexError>) {
    let lines = LineIndex::new(source);
    let mut tokens = Vec::new();
    let mut errors = Vec::new();

    let mut mode = Mode::Normal(NormalToken::lexer(source));
    loop {
        mode = match mode {
            Mode::Normal(mut lexer) => match lexer.next() {
                None => break,
                Some(Ok(NormalToken::Pic)) => {
                    let (line, col) = lines.locate(lexer.span().start);
                    tokens.push(Token { kind: TokenKind::Pic, line, col });
                    // Entra no modo PIC: a partir daqui, só valem as regras
                    // de `PicToken`, até a próxima `PicString`, `Period` ou
                    // `InvalidPicString`.
                    Mode::Pic(lexer.morph())
                }
                Some(Ok(token)) => {
                    let (line, col) = lines.locate(lexer.span().start);
                    handle_normal_token(token, line, col, &mut tokens, &mut errors);
                    Mode::Normal(lexer)
                }
                Some(Err(())) => {
                    // Não deve ocorrer: a regra pega-tudo `InvalidWord` casa
                    // com qualquer palavra sem espaços que não termine em
                    // ponto. Mantido por segurança.
                    let (line, col) = lines.locate(lexer.span().start);
                    errors.push(LexError {
                        message: "erro léxico interno: nenhuma regra casou".into(),
                        line,
                        col,
                    });
                    Mode::Normal(lexer)
                }
            },
            Mode::Pic(mut lexer) => match lexer.next() {
                // O arquivo terminou dentro de uma cláusula PIC sem ponto
                // final; a falta do ponto é um erro de estrutura, e quem
                // acusa isso é o parser, não o léxico.
                None => break,
                Some(Ok(PicToken::Is)) => {
                    let (line, col) = lines.locate(lexer.span().start);
                    tokens.push(Token { kind: TokenKind::Is, line, col });
                    Mode::Pic(lexer)
                }
                Some(Ok(PicToken::PicString(bytes))) => {
                    let (line, col) = lines.locate(lexer.span().start);
                    tokens.push(Token { kind: TokenKind::PicString(to_text(&bytes)), line, col });
                    Mode::Normal(lexer.morph())
                }
                Some(Ok(PicToken::Period)) => {
                    let (line, col) = lines.locate(lexer.span().start);
                    tokens.push(Token { kind: TokenKind::Period, line, col });
                    Mode::Normal(lexer.morph())
                }
                Some(Ok(PicToken::InvalidPicString(bytes))) => {
                    let (line, col) = lines.locate(lexer.span().start);
                    let lexema = to_text(&bytes);
                    errors.push(LexError {
                        message: format!("cadeia PIC inválida '{lexema}'"),
                        line,
                        col,
                    });
                    Mode::Normal(lexer.morph())
                }
                Some(Err(())) => {
                    let (line, col) = lines.locate(lexer.span().start);
                    errors.push(LexError {
                        message: "erro léxico interno: nenhuma regra casou".into(),
                        line,
                        col,
                    });
                    Mode::Normal(lexer.morph())
                }
            },
        };
    }

    (tokens, errors)
}

/// O analisador está sempre em um destes dois modos: NORMAL para o corpo
/// do programa, PIC dentro de uma cláusula `PIC`/`PICTURE`.
enum Mode<'s> {
    Normal(logos::Lexer<'s, NormalToken>),
    Pic(logos::Lexer<'s, PicToken>),
}

fn handle_normal_token(
    token: NormalToken,
    line: usize,
    col: usize,
    tokens: &mut Vec<Token>,
    errors: &mut Vec<LexError>,
) {
    let kind = match token {
        NormalToken::Data => TokenKind::Data,
        NormalToken::Division => TokenKind::Division,
        NormalToken::WorkingStorage => TokenKind::WorkingStorage,
        NormalToken::Section => TokenKind::Section,
        NormalToken::Pic => unreachable!("tratado antes de chamar esta função"),
        NormalToken::Is => TokenKind::Is,
        NormalToken::Redefines => TokenKind::Redefines,
        NormalToken::UnsupportedReserved(bytes) => {
            TokenKind::UnsupportedReserved(to_text(&bytes))
        }
        NormalToken::Number(bytes) => TokenKind::Number(to_text(&bytes)),
        NormalToken::Period => TokenKind::Period,
        NormalToken::Name(bytes) => {
            let nome = to_text(&bytes);
            // Limite de tamanho verificado aqui, e não na expressão regular:
            // a regra continua casando o nome inteiro, e este erro tem uma
            // mensagem própria, mais clara do que uma regex rejeitando.
            if nome.chars().count() > MAX_NAME_LEN {
                errors.push(LexError {
                    message: format!(
                        "nome '{nome}' excede o limite de {MAX_NAME_LEN} caracteres"
                    ),
                    line,
                    col,
                });
            }
            TokenKind::Name(nome)
        }
        NormalToken::InvalidWord(bytes) => {
            let palavra = to_text(&bytes);
            errors.push(LexError { message: diagnose_invalid_word(&palavra), line, col });
            return;
        }
    };
    tokens.push(Token { kind, line, col });
}

/// Reduz a lista de tokens a pares (nome do token, lexema), para comparar
/// com o esperado sem repetir a posição em cada caso de teste. Usada pelos
/// testes deste arquivo e de `diagnostics.rs`.
#[cfg(test)]
fn kinds(tokens: &[Token]) -> Vec<(&'static str, String)> {
    tokens
        .iter()
        .map(|t| match &t.kind {
            TokenKind::Data => ("Data", String::new()),
            TokenKind::Division => ("Division", String::new()),
            TokenKind::WorkingStorage => ("WorkingStorage", String::new()),
            TokenKind::Section => ("Section", String::new()),
            TokenKind::Pic => ("Pic", String::new()),
            TokenKind::Is => ("Is", String::new()),
            TokenKind::Redefines => ("Redefines", String::new()),
            TokenKind::UnsupportedReserved(s) => ("UnsupportedReserved", s.clone()),
            TokenKind::Number(s) => ("Number", s.clone()),
            TokenKind::Name(s) => ("Name", s.clone()),
            TokenKind::Period => ("Period", String::new()),
            TokenKind::PicString(s) => ("PicString", s.clone()),
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn programa_valido_completo() {
        let fonte = b"DATA DIVISION.\nWORKING-STORAGE SECTION.\n\
            77 CONTADOR PIC S9(4).\n01 CLIENTE.\n   05 NOME PIC X(30).\n";
        let (tokens, errors) = lex(fonte);
        assert!(errors.is_empty(), "erros inesperados: {errors:?}");
        assert_eq!(
            kinds(&tokens),
            vec![
                ("Data", "".into()),
                ("Division", "".into()),
                ("Period", "".into()),
                ("WorkingStorage", "".into()),
                ("Section", "".into()),
                ("Period", "".into()),
                ("Number", "77".into()),
                ("Name", "CONTADOR".into()),
                ("Pic", "".into()),
                ("PicString", "S9(4)".into()),
                ("Period", "".into()),
                ("Number", "01".into()),
                ("Name", "CLIENTE".into()),
                ("Period", "".into()),
                ("Number", "05".into()),
                ("Name", "NOME".into()),
                ("Pic", "".into()),
                ("PicString", "X(30)".into()),
                ("Period", "".into()),
            ]
        );
    }

    #[test]
    fn palavras_reservadas_ignoram_maiusculas_e_minusculas() {
        let (tokens, errors) = lex(b"data division.");
        assert!(errors.is_empty());
        assert_eq!(kinds(&tokens), vec![("Data", "".into()), ("Division", "".into()), ("Period", "".into())]);
    }

    #[test]
    fn picture_completo_nao_e_confundido_com_pic() {
        // Casamento mais longo: "PICTURE" (7) vence "PIC" (3), mesmo a
        // reservada `Pic` tendo prioridade maior.
        let (tokens, errors) = lex(b"PICTURE IS S9(7)V99.");
        assert!(errors.is_empty());
        assert_eq!(
            kinds(&tokens),
            vec![
                ("Pic", "".into()),
                ("Is", "".into()),
                ("PicString", "S9(7)V99".into()),
                ("Period", "".into()),
            ]
        );
    }

    #[test]
    fn palavra_pictures_nao_entra_no_modo_pic() {
        // "PICTURES" (8 caracteres) casa mais longo com `Name` do que "PIC"
        // (3) com a reservada `Pic` — por isso é um nome, e não entra no modo PIC.
        let (tokens, errors) = lex(b"PICTURES.");
        assert!(errors.is_empty());
        assert_eq!(kinds(&tokens), vec![("Name", "PICTURES".into()), ("Period", "".into())]);
    }

    #[test]
    fn nome_pode_comecar_com_digito() {
        // Ao contrário de C: "2CONTADOR" é um nome válido em COBOL.
        let (tokens, errors) = lex(b"2A-VIA");
        assert!(errors.is_empty());
        assert_eq!(kinds(&tokens), vec![("Name", "2A-VIA".into())]);
    }

    #[test]
    fn ponto_final_nao_e_confundido_com_ponto_decimal() {
        let (tokens, errors) = lex(b"CLIENTE.");
        assert!(errors.is_empty());
        assert_eq!(kinds(&tokens), vec![("Name", "CLIENTE".into()), ("Period", "".into())]);
    }

    #[test]
    fn numero_aceita_parte_decimal() {
        let (tokens, errors) = lex(b"VALUE 3.5.");
        assert!(errors.is_empty(), "'3.5' não deve gerar erro léxico: {errors:?}");
        assert_eq!(
            kinds(&tokens),
            vec![
                ("UnsupportedReserved", "VALUE".into()),
                ("Number", "3.5".into()),
                ("Period", "".into()),
            ]
        );
    }

    #[test]
    fn comentario_e_ignorado_ate_o_fim_da_linha() {
        let (tokens, errors) = lex(b"*> comentario\nDATA.");
        assert!(errors.is_empty());
        assert_eq!(kinds(&tokens), vec![("Data", "".into()), ("Period", "".into())]);
        assert_eq!((tokens[0].line, tokens[0].col), (2, 1));
    }

    #[test]
    fn nome_no_limite_de_31_caracteres_e_aceito() {
        let nome = "A".repeat(31);
        let (tokens, errors) = lex(nome.as_bytes());
        assert!(errors.is_empty());
        assert_eq!(kinds(&tokens), vec![("Name", nome)]);
    }

    #[test]
    fn nome_com_32_caracteres_excede_o_limite() {
        let nome = "A".repeat(32);
        let (tokens, errors) = lex(nome.as_bytes());
        // O token ainda é reconhecido como Name; só a mensagem de erro muda.
        assert_eq!(kinds(&tokens), vec![("Name", nome)]);
        assert_eq!(errors.len(), 1);
        assert!(errors[0].message.contains("excede o limite de 31 caracteres"));
    }

    #[test]
    fn cadeia_pic_fora_do_subconjunto_e_erro() {
        let (tokens, errors) = lex(b"PIC Q(3).");
        assert_eq!(kinds(&tokens), vec![("Pic", "".into()), ("Period", "".into())]);
        assert_eq!(errors.len(), 1);
        assert!(errors[0].message.contains("cadeia PIC inválida 'Q(3)'"));
    }

    #[test]
    fn cadeia_pic_nao_mistura_x_e_9() {
        let (_, errors) = lex(b"PIC X9.");
        assert_eq!(errors.len(), 1);
        assert!(errors[0].message.contains("cadeia PIC inválida 'X9'"));
    }

    #[test]
    fn modo_pic_volta_ao_normal_depois_da_cadeia() {
        // Depois de reconhecer a PicString, "NOME" volta a ser um Name comum,
        // e não é interpretado como parte da cláusula PIC.
        let (tokens, errors) = lex(b"PIC X NOME");
        assert!(errors.is_empty());
        assert_eq!(
            kinds(&tokens),
            vec![("Pic", "".into()), ("PicString", "X".into()), ("Name", "NOME".into())]
        );
    }
}
