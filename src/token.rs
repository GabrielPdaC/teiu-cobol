//! Tokens do analisador léxico, gerados pela crate `logos` (decisão D6 em
//! `docs/especificacao.md`). A entrada é lida como bytes (`&[u8]`), e não como
//! `&str`, por causa da decisão D8: um byte fora do ASCII deve virar um erro
//! léxico com linha e coluna, em vez de impedir a leitura do arquivo.
//!
//! Existem dois conjuntos de tokens porque a cláusula PIC tem sua própria
//! semântica léxica (decisão D15, "modo PIC"): a mesma palavra (`X`, `9`) é uma
//! cadeia PIC dentro da cláusula e um token inválido fora dela. Isso é o
//! equivalente, em `logos`, das *start conditions* do Flex — o léxico troca de
//! `NormalToken` para `PicToken` com `Lexer::morph` ao reconhecer `Pic`, e volta
//! ao reconhecer `PicString`, `InvalidPicString` ou `Period` (ver `lexer.rs`).
//!
//! Os nomes das variantes seguem a tabela 4 de `docs/especificacao.md`.

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

    /// Entra no modo PIC (ver `lexer.rs`).
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

/// Tokens reconhecidos dentro da cláusula PIC (modo PIC).
#[derive(Logos, Debug, Clone, PartialEq, Eq)]
#[logos(utf8 = false)]
#[logos(skip r"[ \t\r\n]+")]
#[logos(skip(r"\*>[^\n]*", allow_greedy = true))]
pub enum PicToken {
    #[regex("IS", priority = 4, ignore(case))]
    Is,

    /// Cadeia PIC do subconjunto "espelho estrito" (decisão D4, D16): `X`
    /// (char), `9`/`S9` (int) ou `9V9`/`S9V9` (float), sem misturar `X` e `9`.
    /// `\(0*[1-9][0-9]*\)` é a subexpressão `COUNT` da seção 4.2 da
    /// especificação: a contagem entre parênteses, como em `X(30)`; não aceita
    /// `(0)`, mas aceita zeros à esquerda, como em `9(05)`.
    #[regex(
        r"(X(\(0*[1-9][0-9]*\))?)+|S?((9(\(0*[1-9][0-9]*\))?)+(V(9(\(0*[1-9][0-9]*\))?)*)?|V(9(\(0*[1-9][0-9]*\))?)+)",
        |lex| lex.slice().to_owned(),
        priority = 3,
        ignore(case)
    )]
    PicString(Vec<u8>),

    #[regex(r"\.", priority = 2)]
    Period,

    /// Regra pega-tudo de erro do modo PIC, análoga a `NormalToken::InvalidWord`.
    #[regex(r"[^ \t\r\n]*[^ \t\r\n.]", |lex| lex.slice().to_owned(), priority = 1)]
    InvalidPicString(Vec<u8>),
}
