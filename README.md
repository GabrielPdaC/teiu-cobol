# teiu-cobol

Analisador léxico e sintático da seção de declarações de COBOL, escrito em Rust.
Trabalho do Grau A da disciplina de Compiladores (Unisinos).

A especificação dos tokens, a gramática e as decisões de projeto estão em
[docs/especificacao.md](docs/especificacao.md).

## Executar

Requer Rust (stable, edição 2024). As dependências e o motivo de cada uma estão
na decisão D6 da especificação.

```sh
cargo run -- exemplos/validos/01-cliente.cob
```

Códigos de saída: `0` sem erros, `1` com erros no programa analisado,
`2` erro de uso ou de leitura do arquivo.

## Estrutura

```
src/                 código-fonte do analisador
docs/                especificação e decisões de projeto
exemplos/validos/    programas que devem ser aceitos
exemplos/invalidos/  programas com erros léxicos e de estrutura
```
