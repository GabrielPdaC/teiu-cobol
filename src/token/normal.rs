//! Tokens reconhecidos fora da cláusula PIC (modo NORMAL).

use logos::Logos;

/// Tokens reconhecidos fora da cláusula PIC.
#[derive(Logos, Debug, Clone, PartialEq, Eq)]
#[logos(utf8 = false)]
#[logos(skip r"[ \t\r\n]+")]
#[logos(skip(r"\*>[^\n]*", allow_greedy = true))]
pub enum NormalToken {
    #[regex("DATA", priority = 4, ignore(case))]
    Data,

    #[regex("DIVISION", priority = 4, ignore(case))]
    Division,

    #[regex("WORKING-STORAGE", priority = 4, ignore(case))]
    WorkingStorage,

    #[regex("SECTION", priority = 4, ignore(case))]
    Section,

    /// Entra no modo PIC (ver `crate::lexer`).
    #[regex("PIC|PICTURE", priority = 4, ignore(case))]
    Pic,

    #[regex("IS", priority = 4, ignore(case))]
    Is,

    /// Início da cláusula `REDEFINES` (decisão D24): faz um item ocupar o
    /// mesmo espaço de outro já declarado, em vez de um espaço novo.
    #[regex("REDEFINES", priority = 4, ignore(case))]
    Redefines,

    /// Palavras reservadas fora do escopo desta etapa (decisão D5, D18):
    /// reconhecidas para virarem erro de estrutura, e não erro léxico.
    #[regex(
        "VALUE|USAGE|OCCURS|FILLER",
        |lex| lex.slice().to_owned(),
        priority = 4,
        ignore(case)
    )]
    UnsupportedReserved(Vec<u8>),

    /// Nível (01-49, 77) ou número decimal em posição inválida; a faixa é
    /// verificada pelo analisador sintático (decisão D13, D14).
    #[regex(r"[0-9]+(\.[0-9]+)?", |lex| lex.slice().to_owned(), priority = 3)]
    Number(Vec<u8>),

    /// Nome de dado. Ao contrário de C, pode começar com dígito, desde que
    /// tenha ao menos uma letra (decisão D4, seção 4.1 da especificação).
    #[regex(
        r"([0-9]+-+)*[0-9]*[A-Za-z]([A-Za-z0-9-]*[A-Za-z0-9])?",
        |lex| lex.slice().to_owned(),
        priority = 2
    )]
    Name(Vec<u8>),

    #[regex(r"\.", priority = 2)]
    Period,

    /// Regra pega-tudo de erro (decisão D12): casa com qualquer palavra sem
    /// espaços que não termine em ponto. Como tem a menor prioridade, só vence
    /// quando casa um trecho mais longo do que qualquer token válido — ou seja,
    /// quando a palavra inteira é inválida.
    #[regex(r"[^ \t\r\n]*[^ \t\r\n.]", |lex| lex.slice().to_owned(), priority = 1)]
    InvalidWord(Vec<u8>),
}
