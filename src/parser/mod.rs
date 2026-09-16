//! Analisador sintático: consome os tokens de `crate::lexer` e produz a
//! tabela de símbolos (`crate::symbols`) e os erros de estrutura. Dividido
//! em três arquivos, um por responsabilidade:
//!
//! - [`grammar`] — lê a gramática de declarações: cabeçalho, entrada,
//!   cláusulas `PIC` e `REDEFINES`. A pergunta que ele responde é "os
//!   tokens estão na ordem certa?".
//! - [`hierarchy`] — verifica quem é filho de quem, com uma pilha. Não dá
//!   pra saber isso só pela ordem dos tokens (dois `entrada` seguidos são
//!   sintaticamente iguais, sejam irmãos ou pai e filho), por isso fica de
//!   fora da gramática e roda depois, num segundo passo.
//! - [`redefines`] — verifica a regra de posição da cláusula `REDEFINES`:
//!   o alvo precisa existir, ser elementar, e a entrada precisa vir logo
//!   depois dele (ou de outra redefinição do mesmo alvo).
//!
//! As três partes compartilham o mesmo `struct Parser`, definido aqui, cada
//! uma com o seu próprio bloco `impl Parser` no arquivo correspondente — é
//! assim que um método de `hierarchy.rs` consegue chamar um método de
//! `redefines.rs` no mesmo `self`, sem precisar de nenhum parâmetro extra
//! entre eles (ver `check_hierarchy`, que chama `check_redefines` no meio).

mod grammar;
mod hierarchy;
mod redefines;

use crate::lexer::Token;
use crate::symbols::Symbol;

/// Um erro de estrutura, sempre com linha e coluna.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParseError {
    pub message: String,
    pub line: usize,
    pub col: usize,
}

/// Analisa a sequência de tokens e devolve a tabela de símbolos e os erros
/// de estrutura encontrados. Duas etapas, nesta ordem: primeiro a gramática
/// (`grammar.rs`, consome todos os tokens e monta os símbolos), depois a
/// hierarquia (`hierarchy.rs`, que já chama a verificação de `REDEFINES`
/// de `redefines.rs` por dentro).
pub fn parse(tokens: &[Token]) -> (Vec<Symbol>, Vec<ParseError>) {
    let mut parser = Parser { tokens, pos: 0, errors: Vec::new(), symbols: Vec::new() };
    parser.parse_header();
    while parser.peek().is_some() {
        parser.parse_entry();
    }
    parser.check_hierarchy();
    (parser.symbols, parser.errors)
}

/// Estado compartilhado pelas três partes: a posição atual na lista de
/// tokens, os erros acumulados e os símbolos já montados.
struct Parser<'t> {
    tokens: &'t [Token],
    pos: usize,
    errors: Vec<ParseError>,
    symbols: Vec<Symbol>,
}

/// Roda o léxico e o sintático juntos, e falha o teste se sobrar erro
/// léxico — usado pelos testes das três partes, para começarem já com uma
/// lista de tokens válida.
#[cfg(test)]
fn parse_source(source: &[u8]) -> (Vec<Symbol>, Vec<ParseError>) {
    let (tokens, lex_errors) = crate::lexer::lex(source);
    assert!(lex_errors.is_empty(), "fonte não deveria ter erro léxico: {lex_errors:?}");
    parse(&tokens)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::symbols::SymbolKind;

    /// Teste de integração: um programa que passa pelas três partes do
    /// parser (gramática, hierarquia e o cálculo de tamanho dos grupos).
    /// Os testes de cada erro específico ficam no arquivo responsável por
    /// aquele erro.
    #[test]
    fn programa_valido_com_grupo_e_item_isolado() {
        let fonte = b"DATA DIVISION.\nWORKING-STORAGE SECTION.\n\
            77 CONTADOR PIC S9(4).\n01 CLIENTE.\n   05 NOME PIC X(30).\n   05 SALDO PIC S9(7)V99.\n";
        let (symbols, errors) = parse_source(fonte);
        assert!(errors.is_empty(), "erros inesperados: {errors:?}");
        assert_eq!(symbols.len(), 4);

        let contador = &symbols[0];
        assert_eq!(contador.name, "CONTADOR");
        assert_eq!(contador.kind, SymbolKind::Int { signed: true, len: 4 });
        assert_eq!(contador.parent, None);

        let cliente = &symbols[1];
        assert_eq!(cliente.kind, SymbolKind::Group);
        assert_eq!(cliente.size_bytes, 30 + 9); // NOME + SALDO

        let nome = &symbols[2];
        assert_eq!(nome.parent.as_deref(), Some("CLIENTE"));
        assert_eq!(nome.kind, SymbolKind::Char { len: 30 });

        let saldo = &symbols[3];
        assert_eq!(saldo.kind, SymbolKind::Float { signed: true, int_len: 7, frac_len: 2 });
    }
}
