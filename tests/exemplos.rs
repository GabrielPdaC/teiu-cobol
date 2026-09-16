//! Testes de integração (F5): roda o léxico e o sintático sobre cada
//! programa de `exemplos/` e confere contra o resultado esperado — a mesma
//! verificação que o relatório da disciplina comenta. Ver
//! `exemplos/RESULTADOS.md` para a explicação de cada caso.

use std::fs;

use teiu_cobol::{lexer, parser};

enum Esperado {
    /// Programa válido: zero erros, com a quantidade de símbolos que a
    /// tabela de símbolos deve ter ao final.
    Valido { simbolos: usize },
    /// Programa inválido: ao menos um erro, e cada trecho de `contem`
    /// precisa aparecer em alguma mensagem (léxica ou de estrutura).
    Invalido { contem: &'static [&'static str] },
}

struct Caso {
    arquivo: &'static str,
    esperado: Esperado,
}

const CASOS: &[Caso] = &[
    Caso {
        arquivo: "exemplos/validos/01-cliente.cob",
        // 77 CONTADOR, 01 CLIENTE, 05 CNPJ, 05 CPF, 05 NOME, 05 CONTA
        // (grupo aninhado), 10 SALDO, 10 LIMITE.
        esperado: Esperado::Valido { simbolos: 8 },
    },
    Caso {
        arquivo: "exemplos/validos/02-cpf-cnpj-redefines.cob",
        // 01 PESSOA, 05 CPF-CNPJ, 05 CPF, 05 CNPJ.
        esperado: Esperado::Valido { simbolos: 4 },
    },
    Caso { arquivo: "exemplos/validos/03-item-isolado.cob", esperado: Esperado::Valido { simbolos: 1 } },
    Caso {
        arquivo: "exemplos/validos/04-nomes-com-digito-inicial.cob",
        // 01 REGISTRO, 05 2A-VIA, 05 3-CAMPO.
        esperado: Esperado::Valido { simbolos: 3 },
    },
    Caso {
        arquivo: "exemplos/invalidos/01-caractere-invalido.cob",
        esperado: Esperado::Invalido { contem: &["caractere '@' não permitido"] },
    },
    Caso {
        arquivo: "exemplos/invalidos/02-nomes-consecutivos.cob",
        esperado: Esperado::Invalido { contem: &["esperava '.' depois de 'A'"] },
    },
    Caso {
        arquivo: "exemplos/invalidos/03-cadeia-pic-invalida.cob",
        esperado: Esperado::Invalido { contem: &["cadeia PIC inválida 'Q(3)'"] },
    },
    Caso {
        arquivo: "exemplos/invalidos/04-redefines-fora-de-posicao.cob",
        esperado: Esperado::Invalido { contem: &["deve vir logo depois de 'CPF-CNPJ'"] },
    },
    Caso {
        arquivo: "exemplos/invalidos/05-value-nao-suportado.cob",
        esperado: Esperado::Invalido { contem: &["cláusula 'VALUE' não é suportada"] },
    },
    Caso {
        arquivo: "exemplos/invalidos/06-nivel-77-com-subordinado.cob",
        esperado: Esperado::Invalido { contem: &["precisa estar subordinado"] },
    },
];

#[test]
fn exemplos_correspondem_ao_esperado() {
    for caso in CASOS {
        let source = fs::read(caso.arquivo)
            .unwrap_or_else(|e| panic!("não consegui ler '{}': {e}", caso.arquivo));
        let (tokens, lex_errors) = lexer::lex(&source);
        let (symbols, parse_errors) = parser::parse(&tokens);
        let mensagens: Vec<&str> = lex_errors
            .iter()
            .map(|e| e.message.as_str())
            .chain(parse_errors.iter().map(|e| e.message.as_str()))
            .collect();

        match &caso.esperado {
            Esperado::Valido { simbolos } => {
                assert!(
                    mensagens.is_empty(),
                    "{} deveria ser válido, mas teve erro(s): {mensagens:?}",
                    caso.arquivo
                );
                assert_eq!(
                    symbols.len(),
                    *simbolos,
                    "{}: número de símbolos diferente do esperado",
                    caso.arquivo
                );
            }
            Esperado::Invalido { contem } => {
                assert!(!mensagens.is_empty(), "{} deveria ter erro, mas não teve nenhum", caso.arquivo);
                for trecho in *contem {
                    assert!(
                        mensagens.iter().any(|m| m.contains(trecho)),
                        "{}: esperava uma mensagem contendo {trecho:?}; mensagens encontradas: {mensagens:?}",
                        caso.arquivo
                    );
                }
            }
        }
    }
}

/// Garante que a suíte cobre a pasta inteira: todo `.cob` em `exemplos/`
/// precisa estar listado em `CASOS`, senão um exemplo novo pode ficar sem
/// verificação sem ninguém perceber.
#[test]
fn todo_exemplo_esta_coberto_por_um_caso() {
    let listados: Vec<&str> = CASOS.iter().map(|c| c.arquivo).collect();
    for pasta in ["exemplos/validos", "exemplos/invalidos"] {
        let entradas = fs::read_dir(pasta).unwrap_or_else(|e| panic!("não consegui ler '{pasta}': {e}"));
        for entrada in entradas {
            let caminho = entrada.expect("entrada de diretório inválida").path();
            if caminho.extension().is_none_or(|ext| ext != "cob") {
                continue;
            }
            let caminho = caminho.to_string_lossy().replace('\\', "/");
            assert!(
                listados.iter().any(|l| *l == caminho),
                "{caminho} existe em {pasta}, mas não está em CASOS (tests/exemplos.rs)"
            );
        }
    }
}

/// Requisito do enunciado (seção 3): no mínimo três programas válidos e
/// três com erros.
#[test]
fn ha_pelo_menos_tres_validos_e_tres_invalidos() {
    let validos = CASOS.iter().filter(|c| matches!(c.esperado, Esperado::Valido { .. })).count();
    let invalidos = CASOS.iter().filter(|c| matches!(c.esperado, Esperado::Invalido { .. })).count();
    assert!(validos >= 3, "só há {validos} exemplos válidos, o enunciado pede no mínimo 3");
    assert!(invalidos >= 3, "só há {invalidos} exemplos inválidos, o enunciado pede no mínimo 3");
}
