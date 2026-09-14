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

Para rodar os testes automatizados (léxico, sintático e os programas de
`exemplos/`):

```sh
cargo test
```

## Estrutura

```
src/main.rs           linha de comando e impressão de tokens/símbolos/erros
src/token.rs          tokens do logos (NormalToken e PicToken)
src/lexer.rs          troca de modo, linha/coluna, diagnósticos léxicos e testes
src/parser.rs         gramática de declarações, hierarquia de níveis e testes
src/symbols.rs        tabela de símbolos e leitura das cadeias PIC
tests/exemplos.rs     testa todo programa de exemplos/ contra o resultado esperado
docs/                 especificação e decisões de projeto
exemplos/validos/     programas que devem ser aceitos
exemplos/invalidos/   programas com erros léxicos e de estrutura
exemplos/RESULTADOS.md  saídas comentadas, caso a caso, para o relatório
```
