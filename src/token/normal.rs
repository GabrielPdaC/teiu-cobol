//! Tokens reconhecidos fora de uma cláusula `PIC` (modo NORMAL).
//!
//! Cada regra tem uma prioridade: quando duas regras casam o mesmo número
//! de caracteres, vence a de prioridade mais alta. Quando uma casa mais
//! caracteres que a outra, vence sempre a mais longa — a prioridade só
//! desempata.

use logos::Logos;

#[derive(Logos, Debug, Clone, PartialEq, Eq)]
#[logos(utf8 = false)]
#[logos(skip r"[ \t\r\n]+")] // espaço, tabulação, quebra de linha: ignorados
#[logos(skip(r"\*>[^\n]*", allow_greedy = true))] // comentário: `*>` até o fim da linha
pub enum NormalToken {
    #[regex("DATA", priority = 4, ignore(case))] // palavra reservada "DATA"
    Data,

    #[regex("DIVISION", priority = 4, ignore(case))] // palavra reservada "DIVISION"
    Division,

    #[regex("WORKING-STORAGE", priority = 4, ignore(case))] // palavra reservada "WORKING-STORAGE"
    WorkingStorage,

    #[regex("SECTION", priority = 4, ignore(case))] // palavra reservada "SECTION"
    Section,

    /// Início de uma cláusula PIC. Ao ler este token, o léxico troca para
    /// os tokens de `crate::token::PicToken` (ver `crate::lexer`).
    #[regex("PIC|PICTURE", priority = 4, ignore(case))]
    Pic,

    #[regex("IS", priority = 4, ignore(case))] // palavra reservada opcional em "PIC IS ..."
    Is,

    /// Início da cláusula `REDEFINES`: o item declarado a seguir passa a
    /// ocupar o mesmo espaço de outro item já declarado, em vez de reservar
    /// espaço novo.
    #[regex("REDEFINES", priority = 4, ignore(case))]
    Redefines,

    /// Palavras reservadas de COBOL que este analisador reconhece mas não
    /// implementa. `VALUE`, por exemplo, inicializa um item — aqui as
    /// declarações não têm inicialização, então usá-la é sempre erro.
    #[regex(
        "VALUE|USAGE|OCCURS|FILLER",
        |lex| lex.slice().to_owned(),
        priority = 4,
        ignore(case)
    )]
    UnsupportedReserved(Vec<u8>),

    /// Um nível (`01`, `77`) ou um número qualquer — o parser confere se o
    /// valor é mesmo um nível válido (01-49 ou 77), porque o léxico sozinho
    /// não sabe em que posição da declaração o número está.
    /// Aceita: "01", "77", "3.5". Rejeita: "3." (ponto sem dígito depois),
    /// ".5" (falta o dígito antes do ponto).
    #[regex(r"[0-9]+(\.[0-9]+)?", |lex| lex.slice().to_owned(), priority = 3)]
    Number(Vec<u8>),

    /// Nome de um item de dado. Pode começar com dígito (diferente de C),
    /// mas precisa ter ao menos uma letra em algum ponto, e não pode
    /// começar nem terminar com hífen.
    /// Aceita: "CONTADOR", "2A-VIA", "WS-1". Rejeita: "123", "-CONTA", "CONTA-".
    #[regex(
        r"([0-9]+-+)*[0-9]*[A-Za-z]([A-Za-z0-9-]*[A-Za-z0-9])?",
        |lex| lex.slice().to_owned(),
        priority = 2
    )]
    Name(Vec<u8>),

    #[regex(r"\.", priority = 2)] // ponto final, encerra uma declaração
    Period,

    /// Qualquer palavra que não bateu com nenhuma regra acima — sempre um
    /// erro léxico. Tem a prioridade mais baixa de propósito: só "vence"
    /// quando é mais longa que qualquer token válido, o que consome a
    /// palavra inválida inteira (ex.: "-CONTA") numa mensagem só, em vez
    /// de um erro por caractere.
    #[regex(r"[^ \t\r\n]*[^ \t\r\n.]", |lex| lex.slice().to_owned(), priority = 1)]
    InvalidWord(Vec<u8>),
}
