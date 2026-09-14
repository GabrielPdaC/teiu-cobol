//! teiu-cobol: analisador léxico e sintático da seção de declarações de COBOL.
//!
//! Uso: `teiu-cobol <arquivo.cob>`

use std::env;
use std::fs;
use std::process::ExitCode;

/// Código de saída para erro de uso ou de leitura do arquivo (decisão D9).
const SAIDA_ERRO_DE_USO: u8 = 2;

fn main() -> ExitCode {
    let argumentos: Vec<String> = env::args().collect();

    let Some(caminho) = argumentos.get(1) else {
        eprintln!("uso: teiu-cobol <arquivo.cob>");
        return ExitCode::from(SAIDA_ERRO_DE_USO);
    };

    // Lido como bytes, e não como String, para que um caractere fora do ASCII
    // vire erro léxico com número de linha em vez de falha de leitura (decisão D8).
    let fonte: Vec<u8> = match fs::read(caminho) {
        Ok(bytes) => bytes,
        Err(erro) => {
            eprintln!("erro: não foi possível ler '{caminho}': {erro}");
            return ExitCode::from(SAIDA_ERRO_DE_USO);
        }
    };

    println!("arquivo: {caminho} ({} bytes)", fonte.len());
    ExitCode::SUCCESS
}
