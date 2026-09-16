//! A gramática de declarações em si (seção 5 da especificação), implementada
//! como analisador descendente recursivo (decisão D7):
//!
//! ```text
//! programa            = cabecalho , { entrada } ;
//! cabecalho           = "DATA" , "DIVISION" , "." , "WORKING-STORAGE" , "SECTION" , "." ;
//! entrada             = NIVEL , NOME , [ clausula_redefines ] , [ clausula_pic ] , "." ;
//! clausula_redefines  = "REDEFINES" , NOME ;
//! clausula_pic        = ( "PIC" | "PICTURE" ) , [ "IS" ] , CADEIA_PIC ;
//! ```
//!
//! Cada regra acima virou uma função abaixo, que olha o próximo token,
//! decide o que fazer, e consome o que reconheceu. A pergunta que este
//! arquivo responde é só "os tokens estão na ordem certa?" — quem é filho
//! de quem (a hierarquia de níveis) é responsabilidade de `hierarchy.rs`.

use crate::lexer::{Token, TokenKind};
use crate::symbols::{analyze_pic, Symbol, SymbolKind};

use super::{ParseError, Parser};

/// Um item ainda sendo montado, antes de virar `Symbol` (falta calcular o
/// tamanho em bytes dos grupos, que depende dos filhos — ver `hierarchy.rs`).
struct RawEntry {
    name: String,
    level: u32,
    pic: Option<String>,
    redefines: Option<String>,
    parent: Option<String>,
    line: usize,
}

impl<'t> Parser<'t> {
    pub(super) fn peek(&self) -> Option<&'t Token> {
        self.tokens.get(self.pos)
    }

    /// Posição a usar num erro quando a entrada acabou: a do último token
    /// lido, ou (1, 1) se o arquivo estiver vazio.
    fn eof_position(&self) -> (usize, usize) {
        self.tokens.last().map(|t| (t.line, t.col)).unwrap_or((1, 1))
    }

    fn error(&mut self, message: impl Into<String>, line: usize, col: usize) {
        self.errors.push(ParseError { message: message.into(), line, col });
    }

    /// Recuperação em modo pânico: avança até consumir o próximo `Period`,
    /// para a próxima `entrada` começar limpa (D22).
    fn recover_to_period(&mut self) {
        while let Some(token) = self.peek() {
            self.pos += 1;
            if token.kind == TokenKind::Period {
                break;
            }
        }
    }

    pub(super) fn parse_header(&mut self) {
        const ESPERADO: [TokenKind; 6] = [
            TokenKind::Data,
            TokenKind::Division,
            TokenKind::Period,
            TokenKind::WorkingStorage,
            TokenKind::Section,
            TokenKind::Period,
        ];
        let start = self.pos;
        for kind in &ESPERADO {
            match self.peek() {
                Some(token) if &token.kind == kind => self.pos += 1,
                Some(token) => {
                    self.error(
                        format!(
                            "faltou o cabeçalho 'DATA DIVISION.' e 'WORKING-STORAGE SECTION.': \
                             encontrado {}",
                            describe(&token.kind)
                        ),
                        token.line,
                        token.col,
                    );
                    self.pos = start;
                    return;
                }
                None => {
                    let (line, col) = self.eof_position();
                    self.error("faltou o cabeçalho 'DATA DIVISION.' e 'WORKING-STORAGE SECTION.'", line, col);
                    self.pos = start;
                    return;
                }
            }
        }
    }

    pub(super) fn parse_entry(&mut self) {
        let Some(level_token) = self.peek() else { return };
        let TokenKind::Number(level_text) = &level_token.kind else {
            self.error(
                format!(
                    "esperava um nível de declaração (01-49 ou 77), mas encontrou {}",
                    describe(&level_token.kind)
                ),
                level_token.line,
                level_token.col,
            );
            self.recover_to_period();
            return;
        };
        let level_text = level_text.clone();
        let (level_line, level_col) = (level_token.line, level_token.col);
        self.pos += 1;

        let level: u32 = match level_text.parse() {
            Ok(n) if n == 77 || (1..=49).contains(&n) => n,
            _ => {
                self.error(
                    format!("nível '{level_text}' inválido: use 01-49 ou 77"),
                    level_line,
                    level_col,
                );
                self.recover_to_period();
                return;
            }
        };

        let name = match self.peek() {
            Some(Token { kind: TokenKind::Name(name), .. }) => {
                let name = name.clone();
                self.pos += 1;
                name
            }
            Some(token) => {
                self.error(
                    format!(
                        "esperava um nome de dado depois do nível {level_text}, mas encontrou {}",
                        describe(&token.kind)
                    ),
                    token.line,
                    token.col,
                );
                self.recover_to_period();
                return;
            }
            None => {
                let (line, col) = self.eof_position();
                self.error(
                    format!("esperava um nome de dado depois do nível {level_text}"),
                    line,
                    col,
                );
                return;
            }
        };

        let redefines = match self.parse_redefines_clause(&name) {
            Ok(redefines) => redefines,
            Err(()) => return,
        };

        let pic = match self.parse_pic_clause(&name) {
            Ok(pic) => pic,
            Err(()) => return, // erro já registrado; recuperação já feita
        };

        if let Some(Token { kind: TokenKind::UnsupportedReserved(word), line, col }) = self.peek() {
            self.error(
                format!(
                    "cláusula '{word}' não é suportada nesta etapa: as variáveis são apenas \
                     declaradas, sem inicialização (decisão D5 da especificação)"
                ),
                *line,
                *col,
            );
            self.recover_to_period();
            return;
        }

        match self.peek() {
            Some(Token { kind: TokenKind::Period, .. }) => self.pos += 1,
            Some(token) => {
                self.error(
                    format!(
                        "declaração mal formada: esperava '.' depois de '{name}', mas encontrou {}",
                        describe(&token.kind)
                    ),
                    token.line,
                    token.col,
                );
                self.recover_to_period();
                return;
            }
            None => {
                let (line, col) = self.eof_position();
                self.error(format!("faltou o ponto final da declaração de '{name}'"), line, col);
                return;
            }
        }

        self.symbols.push(build_symbol(RawEntry {
            name,
            level,
            pic,
            redefines,
            parent: None, // preenchido em hierarchy.rs
            line: level_line,
        }));
    }

    /// Lê `[ "REDEFINES" NOME ]`. Devolve `Ok(None)` quando não há cláusula
    /// `REDEFINES` (a maioria dos itens). A validação de que o alvo existe e
    /// vem no lugar certo (seção 6.1) fica para `redefines.rs`, porque
    /// depende de itens que ainda não foram lidos neste ponto do arquivo.
    fn parse_redefines_clause(&mut self, name: &str) -> Result<Option<String>, ()> {
        if !matches!(self.peek(), Some(Token { kind: TokenKind::Redefines, .. })) {
            return Ok(None);
        }
        self.pos += 1;
        match self.peek() {
            Some(Token { kind: TokenKind::Name(alvo), .. }) => {
                let alvo = alvo.clone();
                self.pos += 1;
                Ok(Some(alvo))
            }
            Some(token) => {
                self.error(
                    format!(
                        "esperava o nome do item redefinido depois de 'REDEFINES' em '{name}', \
                         mas encontrou {}",
                        describe(&token.kind)
                    ),
                    token.line,
                    token.col,
                );
                self.recover_to_period();
                Err(())
            }
            None => {
                let (line, col) = self.eof_position();
                self.error(
                    format!("esperava o nome do item redefinido depois de 'REDEFINES' em '{name}'"),
                    line,
                    col,
                );
                Err(())
            }
        }
    }

    /// Lê `[ ("PIC"|"PICTURE") ["IS"] CADEIA_PIC ]`. Devolve `Ok(None)`
    /// quando não há cláusula PIC (item de grupo em potencial).
    fn parse_pic_clause(&mut self, name: &str) -> Result<Option<String>, ()> {
        if !matches!(self.peek(), Some(Token { kind: TokenKind::Pic, .. })) {
            return Ok(None);
        }
        self.pos += 1;
        if matches!(self.peek(), Some(Token { kind: TokenKind::Is, .. })) {
            self.pos += 1;
        }
        match self.peek() {
            Some(Token { kind: TokenKind::PicString(pic), .. }) => {
                let pic = pic.clone();
                self.pos += 1;
                Ok(Some(pic))
            }
            Some(token) => {
                self.error(
                    format!(
                        "esperava uma cadeia PIC depois de 'PIC' em '{name}', mas encontrou {}",
                        describe(&token.kind)
                    ),
                    token.line,
                    token.col,
                );
                self.recover_to_period();
                Err(())
            }
            None => {
                let (line, col) = self.eof_position();
                self.error(format!("esperava uma cadeia PIC depois de 'PIC' em '{name}'"), line, col);
                Err(())
            }
        }
    }
}

fn build_symbol(entry: RawEntry) -> Symbol {
    let kind = match &entry.pic {
        Some(pic) => analyze_pic(pic),
        None => SymbolKind::Group,
    };
    Symbol {
        name: entry.name,
        level: entry.level,
        kind,
        size_bytes: kind.size_bytes(),
        pic: entry.pic,
        parent: entry.parent,
        redefines: entry.redefines,
        line: entry.line,
    }
}

/// Descreve um token para mensagens de erro em português (seção 9).
fn describe(kind: &TokenKind) -> String {
    match kind {
        TokenKind::Data => "a palavra reservada 'DATA'".into(),
        TokenKind::Division => "a palavra reservada 'DIVISION'".into(),
        TokenKind::WorkingStorage => "a palavra reservada 'WORKING-STORAGE'".into(),
        TokenKind::Section => "a palavra reservada 'SECTION'".into(),
        TokenKind::Pic => "a palavra reservada 'PIC'".into(),
        TokenKind::Is => "a palavra reservada 'IS'".into(),
        TokenKind::Redefines => "a palavra reservada 'REDEFINES'".into(),
        TokenKind::UnsupportedReserved(w) => format!("a palavra reservada '{w}'"),
        TokenKind::Number(n) => format!("o número '{n}'"),
        TokenKind::Name(n) => format!("o nome '{n}'"),
        TokenKind::Period => "o ponto final".into(),
        TokenKind::PicString(s) => format!("a cadeia PIC '{s}'"),
    }
}

#[cfg(test)]
mod tests {
    use super::super::parse_source;

    #[test]
    fn falta_o_cabecalho() {
        let (_, errors) = parse_source(b"01 CLIENTE PIC X.\n");
        assert_eq!(errors.len(), 1);
        assert!(errors[0].message.contains("faltou o cabeçalho"));
    }

    #[test]
    fn dois_nomes_seguidos_e_erro_de_estrutura() {
        // Análogo a "int a b;" do enunciado.
        let fonte = b"DATA DIVISION.\nWORKING-STORAGE SECTION.\n01 A B PIC X.\n";
        let (_, errors) = parse_source(fonte);
        assert_eq!(errors.len(), 1);
        assert!(errors[0].message.contains("esperava '.' depois de 'A'"), "{:?}", errors[0]);
    }

    #[test]
    fn falta_o_ponto_final() {
        let fonte = b"DATA DIVISION.\nWORKING-STORAGE SECTION.\n01 CLIENTE PIC X(10)\n";
        let (_, errors) = parse_source(fonte);
        assert_eq!(errors.len(), 1);
        assert!(errors[0].message.contains("faltou o ponto final"));
    }

    #[test]
    fn value_e_erro_de_estrutura_nao_lexico() {
        // Literais entre aspas não fazem parte do subconjunto (limitação L2);
        // por isso o teste usa um valor numérico, para isolar o erro de
        // estrutura que a cláusula VALUE em si já basta para gerar.
        let fonte = b"DATA DIVISION.\nWORKING-STORAGE SECTION.\n01 W78-NUMPRG VALUE 1.\n";
        let (_, errors) = parse_source(fonte);
        assert_eq!(errors.len(), 1);
        assert!(errors[0].message.contains("cláusula 'VALUE' não é suportada"));
    }

    #[test]
    fn nivel_fora_da_faixa_e_erro() {
        let fonte = b"DATA DIVISION.\nWORKING-STORAGE SECTION.\n88 X PIC X.\n";
        let (_, errors) = parse_source(fonte);
        assert_eq!(errors.len(), 1);
        assert!(errors[0].message.contains("nível '88' inválido"));
    }

    #[test]
    fn recupera_apos_erro_e_relata_o_resto() {
        let fonte = b"DATA DIVISION.\nWORKING-STORAGE SECTION.\n01 A B PIC X.\n77 CONTADOR PIC 9(4).\n";
        let (symbols, errors) = parse_source(fonte);
        assert_eq!(errors.len(), 1, "só o primeiro item tem erro: {errors:?}");
        assert_eq!(symbols.len(), 1);
        assert_eq!(symbols[0].name, "CONTADOR");
    }
}
