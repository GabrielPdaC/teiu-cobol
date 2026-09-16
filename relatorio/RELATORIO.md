COMPILADORES · TRABALHO DO GRAU A (TGA)

# Relatório do Trabalho do Grau A

> **Pendências antes da entrega**, marcadas com 🔲 ao longo do texto:
> nome completo e RA do autor (seção 1); link do notebook do Colab, que
> depende de publicar o repositório no GitHub (seção 1 e seção 4).

## 1. Identificação

- **Trabalho:** Trabalho do Grau A (TGA) — Analisador léxico e gramática de declarações
- **Dupla:** 🔲 [nome completo] — 🔲 [RA] · trabalho individual
- **Linguagem reconhecida:** COBOL (seção de declarações), no lugar da
  linguagem em estilo C do enunciado — autorizado pelo professor, para que o
  analisador pudesse ser reaproveitado fora da disciplina.
- **Ferramenta:** Rust, com a biblioteca [`logos`](https://docs.rs/logos) para
  o léxico — no lugar de Flex/PLY/Lark, também autorizado pelo professor.
  `logos` compila as regras num autômato finito determinístico e resolve
  ambiguidades por casamento mais longo e prioridade, exatamente como o
  Flex; a justificativa completa está na seção 3.
- **Onde roda:** localmente via `cargo` (Rust estável, edição 2024; ver
  seção 4 para os comandos exatos). 🔲 Notebook do Google Colab: pendente —
  depende de publicar o repositório no GitHub, que ficou combinado para
  depois da entrega desta etapa. Sem dependências de sistema, a única coisa
  que o notebook precisa fazer é instalar o Rust (`rustup`), clonar o
  repositório e rodar `cargo test`/`cargo run`.

## 2. Especificação da linguagem

### 2.1 O que a linguagem cobre

A linguagem reconhecida é o subconjunto de COBOL correspondente à seção de
declarações de dados (`DATA DIVISION` / `WORKING-STORAGE SECTION`), em
formato livre (sem as colunas do COBOL clássico), com base na norma
**ISO/IEC 1989:2014**. A tabela abaixo traduz o enunciado original (estilo
C) para o que foi de fato implementado:

| Enunciado original (estilo C) | Equivalente em COBOL |
|---|---|
| `char` | `PIC X` ou `PIC X(n)` |
| `int` | `PIC 9(n)` ou `PIC S9(n)` |
| `float` | `PIC 9(n)V9(m)` ou `PIC S9(n)V9(m)` |
| `int a, b, c;` (lista) | item de grupo: um `01` com itens `05` subordinados |
| termina em `;` | termina em `.` |
| sem inicialização | sem cláusula `VALUE` |
| espaços ignorados | espaços, tabulações, quebras de linha e comentários `*>` ignorados |
| *(sem equivalente)* | `REDEFINES`: item ocupando o espaço de outro já declarado (acréscimo ao escopo, ver seção 3.3) |

Exemplo de programa aceito, reunindo o essencial do escopo:

```cobol
DATA DIVISION.
WORKING-STORAGE SECTION.
77 CONTADOR       PIC S9(4).
01 CLIENTE.
   05 NOME        PIC X(30).
   05 SALDO       PIC S9(7)V99.
   05 CPF-CNPJ    PIC X(14).
   05 CPF         REDEFINES CPF-CNPJ PIC 9(11).
   05 CNPJ        REDEFINES CPF-CNPJ PIC 9(14).
```

**Rejeitado de propósito**, com erro de estrutura (não léxico, porque são
palavras reservadas legítimas de COBOL): `VALUE`, `USAGE`, `OCCURS`,
`FILLER`, e os níveis `66` e `88`. `VALUE` é a cláusula de inicialização do
COBOL — o enunciado original pede explicitamente "declaração sem
inicialização", e foi mantida fora mesmo sendo trivial de aceitar.

### 2.2 Alfabeto e formato do código-fonte

- Formato **livre**: sem regras de coluna.
- Alfabeto **ASCII**; um byte fora do ASCII só é aceito dentro de
  comentários — em qualquer outro lugar, é erro léxico.
- Maiúsculas e minúsculas equivalentes em palavras reservadas, nomes e
  cadeias PIC.
- Comentário: `*>` no início de uma palavra inicia um comentário até o fim
  da linha.
- Linha e coluna começam em 1; linha conta quebras `\n`, coluna é contada
  em bytes dentro da linha.

### 2.3 Tokens e expressões regulares

Subexpressão auxiliar usada nas cadeias PIC — `COUNT` é a contagem entre
parênteses, como em `X(30)`:

```
COUNT = \(0*[1-9][0-9]*\)
REPX  = X(COUNT)?
REP9  = 9(COUNT)?
```

**Modo normal** (fora de uma cláusula `PIC`):

| Token | Expressão regular | Aceita | Rejeita |
|---|---|---|---|
| `Data` | `DATA` | `DATA`, `data` | — |
| `Division` | `DIVISION` | `DIVISION` | — |
| `WorkingStorage` | `WORKING-STORAGE` | `WORKING-STORAGE` | — |
| `Section` | `SECTION` | `SECTION` | — |
| `Pic` | `PIC\|PICTURE` | `PIC`, `PICTURE` | `PICTURES` (vira `Name`) |
| `Is` | `IS` | `IS` | — |
| `Redefines` | `REDEFINES` | `REDEFINES` | — |
| `UnsupportedReserved` | `VALUE\|USAGE\|OCCURS\|FILLER` | `VALUE` | — |
| `Number` | `[0-9]+(\.[0-9]+)?` | `01`, `77`, `3.5` | `3.`, `.5`, `1-2` |
| `Name` | `([0-9]+-+)*[0-9]*[A-Z]([A-Z0-9-]*[A-Z0-9])?` | `CONTADOR`, `2A-VIA` | `-CONTA`, `CONTA-`, `123` |
| `Period` | `\.` | `.` | — |
| `InvalidWord` | `[^ \t\r\n]*[^ \t\r\n.]` | *(sempre erro)* | `CONTA@X`, `1-2` |

**Modo PIC** (entre `PIC`/`PICTURE` e o próximo `.`):

| Token | Expressão regular | Aceita | Rejeita |
|---|---|---|---|
| `Is` | `IS` | `IS` | — |
| `PicString` | `(REPX)+ \| S?((REP9)+(V(REP9)*)? \| V(REP9)+)` | `X(30)`, `S9(7)V99` | `X9`, `Q(3)`, `9(5).99` |
| `Period` | `\.` | `.` | — |
| `InvalidPicString` | `[^ \t\r\n]*[^ \t\r\n.]` | *(sempre erro)* | `X9`, `Q(3)` |

Mensagens de erro léxico, sempre com linha e coluna
(`erro de léxico [linha L, coluna C]: <descrição>`):

| Situação | Exemplo | Mensagem |
|---|---|---|
| Caractere não permitido | `CONTA@X` | `palavra inválida 'CONTA@X': caractere '@' não permitido (esperado letras, dígitos e hífen)` |
| Hífen no início ou no fim | `-CONTA` | `palavra inválida '-CONTA': nome não pode começar nem terminar com hífen` |
| Palavra sem nenhuma letra | `1-2` | `palavra inválida '1-2': nome precisa de pelo menos uma letra` |
| Nome com mais de 31 caracteres | (32 caracteres) | `nome '...' excede o limite de 31 caracteres` |
| Cadeia PIC fora do subconjunto | `Q(3)`, `X9` | `cadeia PIC inválida 'Q(3)'` |

### 2.4 Gramática das declarações (EBNF)

```ebnf
programa            = cabecalho , { entrada } ;
cabecalho           = "DATA" , "DIVISION" , "." , "WORKING-STORAGE" , "SECTION" , "." ;
entrada             = NIVEL , NOME , [ clausula_redefines ] , [ clausula_pic ] , "." ;
clausula_redefines  = "REDEFINES" , NOME ;
clausula_pic        = ( "PIC" | "PICTURE" ) , [ "IS" ] , CADEIA_PIC ;
```

`NIVEL`, `NOME` e `CADEIA_PIC` são os tokens `Number`, `Name` e `PicString`
da seção 2.3. A gramática é **LL(1)**: em toda posição, o próximo token
já diz qual regra aplicar.

A hierarquia de níveis (quem é filho de quem) **não está** na gramática
acima, de propósito — ver seção 3.1. Em resumo: uma entrada de nível `L`
fecha todo item aberto de nível `≥ L`; o pai é o que sobra no topo depois
disso; item sem `PIC` pode ganhar filhos, item com `PIC` não; o nível `77`
nunca tem pai nem filhos. Rejeitado com erro de estrutura, de propósito:
nível fora de 01-49/77, item de nível 02-49 sem nenhum grupo aberto de
nível menor, item sem `PIC` que nunca ganhou subordinado, e a falta do
ponto final.

## 3. Decisões de projeto

### 3.1 Por que a hierarquia ficou fora da gramática

Dois `entrada` seguidos são sintaticamente idênticos na EBNF da seção 2.4,
sejam eles irmãos (`01` e `01`) ou pai e filho (`01` e `05`) — a diferença
depende do valor dos níveis já vistos antes, o que é **contexto**, não
estrutura da frase. Colocar isso na gramática exigiria uma gramática
sensível a contexto (ou reescrever a EBNF para cada profundidade possível de
aninhamento, o que não escala). A solução foi verificar a hierarquia à
parte, com uma pilha, **depois** que o programa inteiro já foi lido — porque
o tamanho de um grupo só se conhece depois de saber todos os filhos dele.

### 3.2 Ordem das regras léxicas: casamento mais longo + prioridade

O `logos` resolve ambiguidade em duas etapas, na ordem: primeiro o
casamento mais longo (a regra que reconhece o maior trecho a partir da
posição atual vence), e só em caso de empate no tamanho a prioridade
declarada desempata. A tabela de prioridades usada:

| Prioridade | Regras |
|---|---|
| 5 (mais alta) | espaços e comentários (ignorados) |
| 4 | palavras reservadas |
| 3 | `Number`, `PicString` |
| 2 | `Name`, `Period` |
| 1 (mais baixa) | `InvalidWord`, `InvalidPicString` (pega-tudo de erro) |

**O padrão do identificador.** A regra de `Name` foi o ponto mais delicado,
porque em COBOL — ao contrário de C — um nome **pode** começar com dígito
(`2A-VIA` é válido). A regex ficou dividida em três partes:

```
([0-9]+-+)*[0-9]*[A-Z]([A-Z0-9-]*[A-Z0-9])?
```

- `([0-9]+-+)*[0-9]*` permite começar com dígitos, inclusive separados por
  hífen;
- `[A-Z]` garante que exista **ao menos uma letra** em algum ponto do nome
  — essa parte não é opcional, e é o que rejeita `123` ou `1-2`;
- `([A-Z0-9-]*[A-Z0-9])?` permite hífen no meio, mas exige que o nome
  termine em letra ou dígito (nunca em hífen), o que rejeita `CONTA-`.

**Palavras reservadas.** `DATA` casa tanto com a regra da palavra reservada
quanto com a de `Name` — os dois reconhecem 4 caracteres, um empate exato de
tamanho. A prioridade (4 contra 2) desempata a favor da reservada; no Flex,
o mesmo problema se resolveria declarando a regra da palavra reservada antes
da de identificador no arquivo `.l`. Sem essa prioridade, `int` (ou, no
nosso caso, `DATA`) seria reconhecido como identificador comum, o erro
clássico citado no próprio enunciado da disciplina.

**Um caso que só apareceu na prática.** `PIC|PICTURE` é uma alternância
dentro da mesma regra — e ela levanta uma pegadinha sutil: será que o motor
para em `PIC` (o prefixo mais curto) ou lê `PICTURE` inteiro? A resposta
depende do tipo de motor de expressão regular. Antes de implementar em
Rust, as regras foram simuladas em Python para validar casamento mais longo
e prioridade — e a simulação inicial **errou** justamente nisso, reconhecendo
`PICTURE` como `PIC` seguido de sobra, porque o motor `re` do Python usa
*backtracking* (testa as alternativas em ordem e para na primeira que
casa). O `logos`, como o Flex, compila as regras num autômato finito
determinístico, que sempre encontra o casamento **mais longo** possível — e
por isso reconhece `PICTURE` corretamente. Esse achado virou um teste
automatizado (`picture_completo_nao_e_confundido_com_pic`) e uma nota na
especificação, porque é exatamente o tipo de detalhe que só aparece testando
a especificação, e não só lendo-a.

### 3.3 A cláusula `REDEFINES` (acréscimo ao escopo)

Foi pedido, durante o desenvolvimento, um caso de uso real: um campo de
documento que pode ser CPF (11 dígitos) ou CNPJ (14 dígitos), no mesmo
espaço de memória — o padrão `REDEFINES` do COBOL de produção. Isso não
tem equivalente na linguagem estilo C do enunciado original, e foi
acrescentado como decisão de projeto, restrito a itens elementares (com
`PIC`) dos dois lados, para não precisar recalcular o tamanho de uma
subárvore inteira quando um grupo redefine outro (deixado como limitação
conhecida, seção 6).

A regra de posição foi o ponto mais difícil: não basta exigir que a entrada
venha logo depois do alvo — isso impediria `CPF` **e** `CNPJ`
redefinirem os dois o mesmo `CPF-CNPJ`:

```cobol
05 CPF-CNPJ PIC X(14).
05 CPF       REDEFINES CPF-CNPJ PIC 9(11).
05 CNPJ      REDEFINES CPF-CNPJ PIC 9(14).
```

A regra final: a entrada de `REDEFINES` precisa vir logo depois do alvo
**ou** logo depois de outra entrada que já redefina o mesmo alvo. É essa
segunda parte que permite `CNPJ` (fisicamente depois de `CPF`) escrever
`REDEFINES CPF-CNPJ` — o alvo original — em vez de `REDEFINES CPF`, e ainda
assim ser aceito. Como os dois ocupam o mesmo espaço, o tamanho do grupo
conta esse espaço **uma vez só**, pelo maior tamanho entre o alvo e todas
as suas redefinições — no exemplo, `PESSOA` fica com 14 bytes (o maior
entre 14, 11 e 14), não 14+11+14.

### 3.4 Outras decisões relevantes

| Decisão | Justificativa |
|---|---|
| `VALUE` é erro de **estrutura**, não léxico | É palavra reservada válida de COBOL; o léxico a reconhece normalmente, e só o sintático a rejeita, porque o que está fora de escopo é a construção (inicialização), não a palavra em si. |
| Regras pega-tudo de erro (`InvalidWord`, `InvalidPicString`), com a menor prioridade | Pelo casamento mais longo, uma palavra inválida inteira (`-CONTA`) é consumida de uma vez e gera **uma** mensagem, em vez de um erro por caractere seguido de tokens soltos. |
| Token `Number` genérico; a faixa de nível (01-49, 77) é verificada no sintático, não no léxico | O léxico não sabe a posição da palavra na entrada — `10` pode ser um nível ou vir depois de `VALUE`. Assim, um nível como `88` vira erro de estrutura com mensagem específica. |
| Fonte lido como bytes (`&[u8]`), não como texto UTF-8 | O alfabeto é ASCII; um byte fora do ASCII vira erro léxico com linha, em vez de impedir a leitura do arquivo inteiro. |
| Recuperação de erro em modo pânico (avança até o próximo `.`) | Técnica mais simples de recuperação sintática, suficiente para relatar mais de um erro por execução, como pede o enunciado. |
| Limite de 31 caracteres verificado no *callback* do token, não na regex | Uma regex com limite de repetição ficaria ilegível; o *callback* dá uma mensagem específica, mantendo o token reconhecido normalmente. |

A lista completa (25 decisões numeradas, D1–D25) e as limitações conhecidas
(L1–L9) estão em `docs/especificacao.md`, seções 9 e 10, mantidas à parte
do relatório porque são consultadas linha a linha durante o desenvolvimento.

## 4. Implementação

### 4.1 Estrutura dos arquivos

```
src/main.rs      linha de comando (clap) e impressão de tokens/símbolos/erros
src/token.rs     tokens do logos: NormalToken (modo normal) e PicToken (modo PIC)
src/lexer.rs     troca de modo, cálculo de linha/coluna, mensagens de erro léxico
src/parser.rs    gramática (seção 2.4), hierarquia de níveis, REDEFINES
src/symbols.rs   tabela de símbolos e leitura da cadeia PIC
tests/exemplos.rs  roda léxico+sintático sobre exemplos/ e confere o resultado
docs/especificacao.md  especificação completa e as 25 decisões de projeto
exemplos/        10 programas de teste (4 válidos, 6 inválidos)
```

Não há um arquivo `.l`: em vez de gerar o léxico a partir de um arquivo à
parte (como o `flex` faz com um `.l`), o `logos` gera o léxico a partir de
atributos Rust sobre um `enum` — o `token.rs` abaixo **é** o equivalente do
`.l`.

### 4.2 Trechos comentados

**O padrão do identificador, como token do `logos`** (`src/token.rs`):

```rust
/// Nome de dado. Ao contrário de C, pode começar com dígito, desde que
/// tenha ao menos uma letra (seção 3.2).
#[regex(
    r"([0-9]+-+)*[0-9]*[A-Za-z]([A-Za-z0-9-]*[A-Za-z0-9])?",
    |lex| lex.slice().to_owned(),
    priority = 2
)]
Name(Vec<u8>),
```

A regex é a mesma da seção 2.3; `|lex| lex.slice().to_owned()` é a *ação*
do token (o equivalente à ação `{ return NOME; }` de uma regra `.l`); e
`priority = 2` só decide empates de tamanho contra outra regra (seção 3.2).

**Troca de modo (as *start conditions* do Flex, via `logos`)** — em
`src/lexer.rs`, ao reconhecer a palavra reservada `PIC`, o léxico troca de
`enum` de tokens sem reprocessar nada:

```rust
Some(Ok(NormalToken::Pic)) => {
    let (line, col) = lines.locate(lexer.span().start);
    tokens.push(Token { kind: TokenKind::Pic, line, col });
    // Entra no modo PIC: a partir daqui só valem as regras de PicToken.
    Mode::Pic(lexer.morph())
}
```

`lexer.morph()` é um método do `logos`: pega o lexer atual, na posição em
que parou, e devolve um lexer de outro tipo de token apontando pro mesmo
lugar.

**Recuperação de erro em modo pânico** (`src/parser.rs`):

```rust
fn recover_to_period(&mut self) {
    while let Some(token) = self.peek() {
        self.pos += 1;
        if token.kind == TokenKind::Period { break; }
    }
}
```

Chamado por toda função de leitura que encontra algo inesperado: avança o
cursor até consumir o próximo ponto final, para a entrada seguinte começar
numa posição limpa — é o que faz `01 A B PIC X.` gerar **um** erro, sem
travar a leitura do resto do arquivo.

**A regra de posição do `REDEFINES`** (`src/parser.rs`), implementando a
seção 3.3:

```rust
let anterior = i.checked_sub(1).map(|j| &self.symbols[j]);
let encadeia_do_mesmo_alvo = anterior.is_some_and(|a|
    a.name == alvo_nome || a.redefines.as_deref() == Some(&alvo_nome)
);
```

A segunda condição (`a.redefines.as_deref() == Some(&alvo_nome)`) é o que
permite `CNPJ` vir logo depois de `CPF` e ainda ser aceito, porque os dois
apontam para o mesmo alvo original.

### 4.3 Como compilar e executar

Requer Rust estável (edição 2024), sem dependências de sistema além do
`cargo`. Nenhum ajuste é necessário para rodar em outro ambiente (inclusive
Google Colab, uma vez publicado o notebook — seção 1).

```sh
# Compilar
cargo build

# Rodar sobre um programa COBOL
cargo run -- exemplos/validos/01-cliente.cob

# Rodar toda a suíte de testes automatizados (39 testes)
cargo test
```

Código de saída do analisador: `0` sem erro nenhum, `1` com erro no
programa analisado (léxico e/ou de estrutura), `2` erro de uso (argumento
ausente ou arquivo inexistente).

## 5. Testes e resultados

Dez programas de teste em `exemplos/` (quatro válidos, seis inválidos —
acima do mínimo de três/três exigido pelo enunciado), cada um verificado
automaticamente por `tests/exemplos.rs`: os válidos são conferidos contra o
número de símbolos esperado, os inválidos contra o trecho de mensagem de
erro esperado. Um teste extra varre as pastas no disco e falha se achar um
`.cob` sem um caso correspondente no teste — um exemplo novo não pode ficar
sem verificação por esquecimento. A discussão completa, arquivo por
arquivo, está em `exemplos/RESULTADOS.md`; abaixo, a saída real de três
casos representativos.

### 5.1 Um programa válido, com `REDEFINES` e grupo aninhado

`exemplos/validos/01-cliente.cob`, rodado com `cargo run --
exemplos/validos/01-cliente.cob`:

```
=== tabela de símbolos ===
linha  nível nome      categoria  pic        bytes  pai      redefines
4      77    CONTADOR  int        S9(4)      4      -        -
5      1     CLIENTE   group      -          62     -        -
6      5     CNPJ      int        9(14)      14     CLIENTE  -
7      5     CPF       int        9(11)      11     CLIENTE  CNPJ
8      5     NOME      char       X(30)      30     CLIENTE  -
9      5     CONTA     group      -          18     CLIENTE  -
10     10    SALDO     float      S9(7)V99   9      CONTA    -
11     10    LIMITE    float      S9(7)V99   9      CONTA    -

nenhum erro encontrado
[código de saída: 0]
```

`CLIENTE` fica com 62 bytes — 14 (o maior entre `CNPJ` e `CPF`, que dividem
o mesmo espaço) + 30 (`NOME`) + 18 (`CONTA`, o grupo aninhado dentro de
`CLIENTE`, que por sua vez soma `SALDO`+`LIMITE`) — e não 14+11+30+18 = 73,
que seria o resultado sem a regra de `REDEFINES` da seção 3.3.

Este exemplo também revelou um bug durante os testes manuais: a primeira
versão do cálculo de tamanho de grupo excluía qualquer filho que fosse,
ele mesmo, um grupo — `CONTA` nunca entrava na soma de `CLIENTE` (dava 44,
não 62). Só apareceu porque este arquivo foi editado para ter um grupo
dentro de outro grupo; até então, os testes automatizados cobriam só um
nível de aninhamento. Corrigido percorrendo os símbolos de trás para
frente, já que num arquivo COBOL um item é sempre declarado depois do
grupo que o contém — ver decisão de projeto 26 (`docs/especificacao.md`) e
o teste de regressão `grupo_aninhado_conta_no_tamanho_do_grupo_de_fora`.

### 5.2 Um programa inválido — erro de estrutura (o análogo de `int a b;`)

`exemplos/invalidos/02-nomes-consecutivos.cob`:

```cobol
DATA DIVISION.
WORKING-STORAGE SECTION.
01 A B PIC X.
```

```
=== erros ===
erro de estrutura [linha 6, coluna 6]: declaração mal formada: esperava '.' depois de 'A', mas encontrou o nome 'B'
[código de saída: 1]
```

Um erro só, na coluna exata do segundo nome — o léxico não acusa nada
(`A` e `B` são nomes perfeitamente válidos isoladamente); o problema é de
estrutura.

### 5.3 Um programa inválido — nível 77 não pode ter subordinado

`exemplos/invalidos/06-nivel-77-com-subordinado.cob`:

```cobol
DATA DIVISION.
WORKING-STORAGE SECTION.
77 CONTADOR PIC 9(3).
05 FILHO PIC X.
```

```
=== erros ===
erro de estrutura [linha 6, coluna 1]: nível 05 de 'FILHO' precisa estar subordinado a um item de nível menor, e não há nenhum aberto nesse ponto
[código de saída: 1]
```

### 5.4 Comportamento inesperado observado

Dois casos, ambos vindos de erro léxico e estrutural **em cadeia** sobre a
mesma linha (não exatamente um bug, mas um comportamento que exigiu
explicação):

- `exemplos/invalidos/01-caractere-invalido.cob` (`01 CONTA@X PIC X.`) gera
  **dois** erros, não um: o léxico rejeita `CONTA@X` inteiro e não produz
  token nenhum para ele; sem o `NOME` esperado ali, o sintático também
  reclama, ao encontrar `PIC` logo depois do nível. As duas camadas reagem,
  cada uma dentro da sua responsabilidade — é o comportamento correto, mas
  a primeira leitura da saída pode surpreender por não ser uma mensagem só.
- O caso do `PICTURE` versus `PIC` (seção 3.2): a simulação inicial em
  Python reconhecia errado, e só a implementação real (com o autômato do
  `logos`) revelou a diferença entre casamento mais longo com
  *backtracking* e sem ele.

## 6. Conclusão

**O que funcionou bem.** A separação em dois modos léxicos (`NORMAL`/`PIC`)
resolveu de forma limpa o problema de `X` e `9` significarem coisas
diferentes dentro e fora de uma cláusula PIC, sem nenhum código especial
além de trocar qual `enum` de tokens o `logos` está usando. A pilha de
hierarquia generalizou bem: a mesma lógica que valida um grupo simples
validou `REDEFINES` sem alterações, porque a cláusula não muda a
hierarquia, só o cálculo de tamanho.

**Limitações conhecidas, deixadas de propósito** (lista completa, L1–L9, em
`docs/especificacao.md`, seção 10):

- Lista de palavras reservadas parcial — `01 MOVE PIC X.` é aceito, embora
  `MOVE` seja reservada em COBOL.
- `REDEFINES` só é aceito entre itens elementares (com PIC); um grupo
  redefinindo outro grupo não é suportado.
- Um item elementar que recebe um subordinado logo depois não gera uma
  mensagem de erro específica (o item com PIC nunca é empilhado, então o
  nível seguinte procura o pai em outro lugar da pilha).
- Nomes repetidos no mesmo programa não são detectados como erro.

Um arquivo de código COBOL de produção real foi usado durante o
desenvolvimento para testar os limites do analisador (depois de remover as
instruções `COPY`, que dependeriam de um pré-processador não implementado
aqui). Ele usa recursos deliberadamente fora do escopo — literais entre
aspas, PIC editado (com `Z` e vírgula), nível 88, `OCCURS`,
`USAGE`/`COMP`, e até uma extensão orientada a objetos de um dialeto de
fornecedor —, confirmando que a linha traçada pelo escopo "espelho estrito"
é a correta para este trabalho: cobrir tudo isso contrariaria o próprio
objetivo do enunciado de dominar o mecanismo por inteiro, em vez de
espalhar o esforço por um COBOL completo.

**O que faria diferente.** Duas coisas, se houvesse mais tempo: dar ao
`REDEFINES` suporte para grupos (não só itens elementares), o que exigiria
recalcular o tamanho de uma subárvore inteira em vez de um único item; e
tratar a lista de palavras reservadas de forma completa, em vez de restrita
ao subconjunto usado nesta etapa, para pegar casos como `01 MOVE PIC X.`.

**Declaração de uso de inteligência artificial.** Este trabalho foi
desenvolvido com o Claude Code (assistente de desenvolvimento da
Anthropic) como par de programação, ao longo de todas as fases —
especificação dos tokens, léxico, sintático/hierarquia/`REDEFINES`, e
suíte de testes. Decisões de escopo e de projeto foram sempre do autor,
tomadas em conversa com a IA: a escolha da linguagem-alvo (COBOL) e da
ferramenta (Rust/`logos`) já vinham combinadas com o professor; dentro
disso, decisões como manter `VALUE` fora do escopo, ou acrescentar
`REDEFINES` para o caso de uso real de CPF/CNPJ, foram discutidas e
decididas explicitamente pelo autor antes da implementação. Cada etapa foi
revisada antes de avançar para a seguinte (o autor confirmou cada
`git commit` individualmente), e um documento à parte, cobrindo arquivo por
arquivo com trechos de código reais, foi gerado especificamente para o
autor conferir e entender cada linha antes da apresentação. As regras
léxicas foram simuladas fora do Rust (em Python) antes da implementação,
para validar casamento mais longo, prioridade e troca de modo contra casos
de teste manuscritos; a implementação em Rust foi conferida por 39 testes
automatizados. Fontes normativas (regras de nome, perfis de conformidade do
GnuCOBOL, documentação da IBM Enterprise COBOL) foram buscadas e citadas
com URL, em vez de assumidas de memória pela IA.

🔲 *Revise este parágrafo antes de entregar: ele precisa refletir com
exatidão como o trabalho foi feito, porque será cobrado na apresentação.*

## 7. Referências

- ISO/IEC 1989:2014 — *Information technology — Programming languages,
  their environments and system software interfaces — Programming language
  COBOL*. <https://www.iso.org/standard/51416.html>
- Perfil de conformidade *COBOL 2014* do GnuCOBOL.
  <https://github.com/OCamlPro/gnucobol/blob/gcos4gnucobol-3.x/config/cobol2014.conf>
- Perfil de conformidade *COBOL 85* do GnuCOBOL.
  <https://github.com/OCamlPro/gnucobol/blob/gcos4gnucobol-3.x/config/cobol85.conf>
- IBM — *COBOL words with single-byte characters* (IBM Enterprise COBOL for
  z/OS).
  <https://www.ibm.com/docs/en/cobol-zos/6.3.0?topic=literals-cobol-words-single-byte-characters>
- GnuCOBOL Programmer's Guide (regras de formação de palavra, seção 2.1.2).
  <https://gnucobol.sourceforge.io/HTML/gnucobpg.html>
- Documentação da crate `logos`. <https://docs.rs/logos>
- Documentação da crate `clap`. <https://docs.rs/clap>
- `docs/especificacao.md` — especificação completa (tokens, gramática, as
  25 decisões de projeto e as 9 limitações conhecidas), neste repositório.
- `exemplos/RESULTADOS.md` — saída comentada de cada programa de teste,
  neste repositório.
