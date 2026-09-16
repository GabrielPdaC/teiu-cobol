//! teiu-cobol: analisador léxico e sintático da seção de declarações de COBOL.

use std::fs;
use std::path::PathBuf;
use std::process::ExitCode;

use clap::Parser;

use teiu_cobol::lexer::{self, TokenKind};
use teiu_cobol::parser;
use teiu_cobol::symbols::SymbolKind;

/// Código de saída para erro de uso ou de leitura do arquivo. É o mesmo
/// código que o clap usa quando os argumentos são inválidos.
const EXIT_USAGE_ERROR: u8 = 2;
/// Código de saída quando o programa COBOL analisado tem erro (léxico ou
/// de estrutura) — diferente de `EXIT_USAGE_ERROR`, que é falha da
/// ferramenta em si, não do programa analisado.
const EXIT_WITH_ERRORS: u8 = 1;

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
    // vire erro léxico com número de linha em vez de falha de leitura.
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

    let (tokens, lex_errors) = lexer::lex(&source);
    let (symbols, parse_errors) = parser::parse(&tokens);

    println!("=== tokens ===");
    println!("{:<6} {:<6} {:<22} lexema", "linha", "coluna", "token");
    for token in &tokens {
        let (nome, lexema) = describe_token(&token.kind);
        println!("{:<6} {:<6} {nome:<22} {lexema}", token.line, token.col);
    }

    println!("\n=== tabela de símbolos ===");
    println!(
        "{:<6} {:<5} {:<28} {:<10} {:<12} {:<6} {:<20} redefines",
        "linha", "nível", "nome", "categoria", "pic", "bytes", "pai"
    );
    for symbol in &symbols {
        let (categoria, pic) = describe_symbol(symbol);
        println!(
            "{:<6} {:<5} {:<28} {categoria:<10} {pic:<12} {:<6} {:<20} {}",
            symbol.line,
            symbol.level,
            symbol.name,
            symbol.size_bytes,
            symbol.parent.as_deref().unwrap_or("-"),
            symbol.redefines.as_deref().unwrap_or("-")
        );
    }

    let total_erros = lex_errors.len() + parse_errors.len();
    if total_erros == 0 {
        println!("\nnenhum erro encontrado");
        return ExitCode::SUCCESS;
    }

    println!("\n=== erros ===");
    // Os dois tipos de erro são listados juntos, em ordem de linha, porque é
    // assim que um usuário lê o arquivo de cima para baixo.
    let mut erros: Vec<(usize, usize, &str, &str)> = lex_errors
        .iter()
        .map(|e| (e.line, e.col, "léxico", e.message.as_str()))
        .chain(parse_errors.iter().map(|e| (e.line, e.col, "estrutura", e.message.as_str())))
        .collect();
    erros.sort_by_key(|(line, col, ..)| (*line, *col));
    for (line, col, tipo, mensagem) in erros {
        println!("erro de {tipo} [linha {line}, coluna {col}]: {mensagem}");
    }
    ExitCode::from(EXIT_WITH_ERRORS)
}

/// Nome do token, para a coluna "token" da tabela impressa, e o lexema a
/// exibir (o texto fixo do token, quando ele não guarda um lexema próprio).
fn describe_token(kind: &TokenKind) -> (&'static str, String) {
    match kind {
        TokenKind::Data => ("Data", "DATA".into()),
        TokenKind::Division => ("Division", "DIVISION".into()),
        TokenKind::WorkingStorage => ("WorkingStorage", "WORKING-STORAGE".into()),
        TokenKind::Section => ("Section", "SECTION".into()),
        TokenKind::Pic => ("Pic", "PIC".into()),
        TokenKind::Is => ("Is", "IS".into()),
        TokenKind::Redefines => ("Redefines", "REDEFINES".into()),
        TokenKind::UnsupportedReserved(lexema) => ("UnsupportedReserved", lexema.clone()),
        TokenKind::Number(lexema) => ("Number", lexema.clone()),
        TokenKind::Name(lexema) => ("Name", lexema.clone()),
        TokenKind::Period => ("Period", ".".into()),
        TokenKind::PicString(lexema) => ("PicString", lexema.clone()),
    }
}

/// Nome da categoria do item (char/int/float/group) e a cadeia PIC a
/// exibir, quando houver uma (itens de grupo não têm PIC).
fn describe_symbol(symbol: &teiu_cobol::symbols::Symbol) -> (&'static str, String) {
    let categoria = match symbol.kind {
        SymbolKind::Group => "group",
        SymbolKind::Char { .. } => "char",
        SymbolKind::Int { .. } => "int",
        SymbolKind::Float { .. } => "float",
    };
    (categoria, symbol.pic.clone().unwrap_or_else(|| "-".into()))
}
