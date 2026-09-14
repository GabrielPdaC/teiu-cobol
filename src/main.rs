//! teiu-cobol: analisador léxico e sintático da seção de declarações de COBOL.

use std::fs;
use std::path::PathBuf;
use std::process::ExitCode;

use clap::Parser;

use teiu_cobol::lexer::{self, TokenKind};

/// Código de saída para erro de uso ou de leitura do arquivo (decisão D9).
/// É o mesmo código que o clap usa quando os argumentos são inválidos.
const EXIT_USAGE_ERROR: u8 = 2;
/// Código de saída quando o programa analisado tem erros (decisão D9).
const EXIT_LEXICAL_ERRORS: u8 = 1;

/// Analisador léxico e sintático da seção de declarações de COBOL.
#[derive(Parser)]
#[command(version, about)]
struct Cli {
    /// Arquivo-fonte COBOL a ser analisado
    source: PathBuf,
}

fn main() -> ExitCode {
    // Em caso de argumento ausente ou inválido, o clap imprime a ajuda e
    // encerra o programa com o código 2.
    let cli = Cli::parse();

    // Lido como bytes, e não como String, para que um caractere fora do ASCII
    // vire erro léxico com número de linha em vez de falha de leitura (decisão D8).
    let source: Vec<u8> = match fs::read(&cli.source) {
        Ok(bytes) => bytes,
        Err(error) => {
            eprintln!(
                "erro: não foi possível ler '{}': {error}",
                cli.source.display()
            );
            return ExitCode::from(EXIT_USAGE_ERROR);
        }
    };

    let (tokens, errors) = lexer::lex(&source);

    println!("=== tokens ===");
    println!("{:<6} {:<6} {:<22} lexema", "linha", "coluna", "token");
    for token in &tokens {
        let (nome, lexema) = describe(&token.kind);
        println!("{:<6} {:<6} {nome:<22} {lexema}", token.line, token.col);
    }

    if errors.is_empty() {
        println!("\nnenhum erro léxico encontrado");
        ExitCode::SUCCESS
    } else {
        println!("\n=== erros léxicos ===");
        for erro in &errors {
            println!("erro léxico [linha {}, coluna {}]: {}", erro.line, erro.col, erro.message);
        }
        ExitCode::from(EXIT_LEXICAL_ERRORS)
    }
}

/// Nome do token (igual ao das variantes de `docs/especificacao.md`) e o
/// lexema a exibir, quando houver um específico.
fn describe(kind: &TokenKind) -> (&'static str, String) {
    match kind {
        TokenKind::Data => ("Data", "DATA".into()),
        TokenKind::Division => ("Division", "DIVISION".into()),
        TokenKind::WorkingStorage => ("WorkingStorage", "WORKING-STORAGE".into()),
        TokenKind::Section => ("Section", "SECTION".into()),
        TokenKind::Pic => ("Pic", "PIC".into()),
        TokenKind::Is => ("Is", "IS".into()),
        TokenKind::UnsupportedReserved(lexema) => ("UnsupportedReserved", lexema.clone()),
        TokenKind::Number(lexema) => ("Number", lexema.clone()),
        TokenKind::Name(lexema) => ("Name", lexema.clone()),
        TokenKind::Period => ("Period", ".".into()),
        TokenKind::PicString(lexema) => ("PicString", lexema.clone()),
    }
}
