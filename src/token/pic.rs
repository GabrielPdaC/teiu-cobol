//! Tokens reconhecidos dentro de uma cláusula `PIC` (modo PIC): aqui `X` e
//! `9` são símbolos de tipo, não nomes de item — por isso são tokens à
//! parte de `NormalToken`.

use logos::Logos;

#[derive(Logos, Debug, Clone, PartialEq, Eq)]
#[logos(utf8 = false)]
#[logos(skip r"[ \t\r\n]+")] // espaço, tabulação, quebra de linha: ignorados
#[logos(skip(r"\*>[^\n]*", allow_greedy = true))] // comentário: `*>` até o fim da linha
pub enum PicToken {
    #[regex("IS", priority = 4, ignore(case))] // palavra reservada "IS"
    Is,

    /// Uma cadeia PIC: `X`/`X(n)` para texto, `9`/`S9` para inteiro, ou
    /// `9V9`/`S9V9` para decimal (`V` marca a casa decimal, `S` o sinal).
    /// `X` e `9` nunca se misturam na mesma cadeia. `(n)` é uma contagem
    /// de repetição, como em `X(30)` (30 caracteres) ou `9(05)` (5
    /// dígitos — zero à esquerda é aceito, mas `(0)` não é).
    /// Aceita: "X", "X(30)", "S9(7)V99", "V99". Rejeita: "X9" (mistura
    /// tipos), "S" e "SV" (sinal sem nenhum dígito).
    #[regex(
        r"(X(\(0*[1-9][0-9]*\))?)+|S?((9(\(0*[1-9][0-9]*\))?)+(V(9(\(0*[1-9][0-9]*\))?)*)?|V(9(\(0*[1-9][0-9]*\))?)+)",
        |lex| lex.slice().to_owned(),
        priority = 3,
        ignore(case)
    )]
    PicString(Vec<u8>),

    #[regex(r"\.", priority = 2)] // ponto final, encerra a cláusula PIC
    Period,

    /// Qualquer palavra que não é uma cadeia PIC válida — sempre um erro
    /// léxico (ex.: "X9", "Q(3)"). Prioridade mais baixa, pelo mesmo
    /// motivo de `NormalToken::InvalidWord`: só vence quando é mais longa
    /// que qualquer cadeia PIC válida, consumindo a palavra inteira.
    #[regex(r"[^ \t\r\n]*[^ \t\r\n.]", |lex| lex.slice().to_owned(), priority = 1)]
    InvalidPicString(Vec<u8>),
}
