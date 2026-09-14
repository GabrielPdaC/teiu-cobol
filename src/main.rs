//! teiu-cobol: analisador léxico e sintático da seção de declarações de COBOL.

use std::fs;
use std::path::PathBuf;
use std::process::ExitCode;

use clap::Parser;

/// Código de saída para erro de uso ou de leitura do arquivo (decisão D9).
/// É o mesmo código que o clap usa quando os argumentos são inválidos.
const EXIT_USAGE_ERROR: u8 = 2;

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

    println!("arquivo: {} ({} bytes)", cli.source.display(), source.len());
    ExitCode::SUCCESS
}
