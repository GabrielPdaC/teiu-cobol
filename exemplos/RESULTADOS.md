# Resultados dos programas de teste (F5)

Este documento comenta a saída de `teiu-cobol` sobre cada programa de
`exemplos/`, para a seção de testes e análise dos resultados do relatório.
A verificação automatizada dos mesmos casos está em `tests/exemplos.rs`
(`cargo test`); aqui o objetivo é explicar **por quê** cada saída é a
esperada, caso a caso.

Em todos os exemplos, código de saída `0` = nenhum erro, `1` = há erro.
A tabela de tokens é omitida abaixo (fica completa ao rodar
`cargo run -- <arquivo>`); só a tabela de símbolos e os erros são citados.

## Programas válidos

### `validos/01-cliente.cob`

Reúne, num só programa, quase todo o escopo: um `77` isolado, um grupo
(`01`) com filhos, dentro dele uma cláusula `REDEFINES` (`CPF` redefinindo
`CNPJ`), **e um grupo aninhado dentro do grupo** (`CONTA`, com `SALDO` e
`LIMITE` por dentro). Roda sem erro, código `0`.

```
linha  nível nome        categoria  pic          bytes  pai      redefines
4      77    CONTADOR    int        S9(4)        4      -        -
5      1     CLIENTE     group      -            62     -        -
6      5     CNPJ        int        9(14)        14     CLIENTE  -
7      5     CPF         int        9(11)        11     CLIENTE  CNPJ
8      5     NOME        char       X(30)        30     CLIENTE  -
9      5     CONTA       group      -            18     CLIENTE  -
10     10    SALDO       float      S9(7)V99     9      CONTA    -
11     10    LIMITE      float      S9(7)V99     9      CONTA    -
```

`CLIENTE` fica com 62 bytes: 14 (o maior entre `CNPJ` e `CPF`, que dividem
o mesmo espaço — seção 6.1) + 30 (`NOME`) + 18 (`CONTA`, o grupo aninhado,
que por sua vez é 9 de `SALDO` + 9 de `LIMITE`). Sem `REDEFINES` teria sido
14+11+30+18 = 73; a diferença de 11 bytes é exatamente o espaço que `CPF`
deixou de duplicar.

**Achado durante os testes manuais:** a primeira versão do cálculo de
tamanho excluía qualquer item que fosse, ele mesmo, um grupo — então
`CONTA` nunca entrava na soma de `CLIENTE` (dava 44, não 62). O bug só
apareceu porque este exemplo foi editado para ter um grupo dentro de
grupo; os testes automatizados até então só cobriam um nível de
aninhamento. Corrigido percorrendo os símbolos de trás para frente
(decisão D26): como um item é sempre declarado depois do grupo que o
contém, ao chegar na linha do grupo de fora, o grupo de dentro já teve o
tamanho resolvido. Ganhou um teste de regressão dedicado
(`grupo_aninhado_conta_no_tamanho_do_grupo_de_fora`, em
`src/parser/hierarchy.rs`).

### `validos/02-cpf-cnpj-redefines.cob`

O mesmo padrão `REDEFINES` isolado, sem o resto do programa em volta, para
testar só essa cláusula. `PESSOA` fica com 14 bytes — o tamanho de
`CPF-CNPJ`, que é o maior entre os três (14, 11 e 14).

### `validos/03-item-isolado.cob`

O caso de canto mais simples: um único `77`, sem cabeçalho de grupo nenhum
depois dele. Confirma que o programa não exige nenhum grupo — um item
isolado já é um programa completo.

### `validos/04-nomes-com-digito-inicial.cob`

Nomes começando por dígito (`2A-VIA`, `3-CAMPO`): válidos em COBOL, ao
contrário de C — é o primeiro "erro frequente" citado no enunciado (seção
8), e aqui aparece como caso de **aceitação**, não de rejeição, porque é
exatamente o comportamento correto.

## Programas inválidos

### `invalidos/01-caractere-invalido.cob` — erro léxico + efeito em cadeia

```
erro de léxico [linha 5, coluna 4]: palavra inválida 'CONTA@X': caractere '@' não permitido (esperado letras, dígitos e hífen)
erro de estrutura [linha 5, coluna 12]: esperava um nome de dado depois do nível 01, mas encontrou a palavra reservada 'PIC'
```

Dois erros, não um. O léxico rejeita `CONTA@X` inteiro (decisão D12) e não
gera token nenhum para ele; sem o `NOME` esperado ali, o sintático também
reclama, ao ver `PIC` logo depois do nível. É um bom exemplo para a
apresentação: mostra as duas camadas reagindo ao mesmo problema, cada uma
com sua responsabilidade.

### `invalidos/02-nomes-consecutivos.cob` — erro de estrutura

```
erro de estrutura [linha 6, coluna 6]: declaração mal formada: esperava '.' depois de 'A', mas encontrou o nome 'B'
```

O análogo COBOL do `int a b;` do enunciado (seção 8). Um erro só, na coluna
exata do segundo nome — o léxico não acusa nada, porque `A` e `B` são
nomes perfeitamente válidos; o problema é de estrutura (dois nomes sem
separador entre eles).

### `invalidos/03-cadeia-pic-invalida.cob` — erro léxico + efeito em cadeia

```
erro de léxico [linha 5, coluna 10]: cadeia PIC inválida 'Q(3)'
erro de estrutura [linha 5, coluna 14]: esperava uma cadeia PIC depois de 'PIC' em 'X', mas encontrou o ponto final
```

Mesmo padrão do primeiro caso: `Q(3)` não é uma cadeia PIC válida no
subconjunto (só `X` e `9`), o léxico não gera token para ela, e o
sintático encontra o ponto final onde esperava a cadeia.

### `invalidos/04-redefines-fora-de-posicao.cob` — erro de estrutura

```
erro de estrutura [linha 8, coluna 1]: REDEFINES de 'CPF' deve vir logo depois de 'CPF-CNPJ', ou de outro item que já redefina 'CPF-CNPJ' — é assim que CPF e CNPJ, por exemplo, podem redefinir o mesmo campo
```

`NOME` foi inserido entre `CPF-CNPJ` e `CPF REDEFINES CPF-CNPJ`, quebrando
a adjacência exigida (seção 6.1). Puramente léxico, o programa seria
válido — o erro só aparece porque o sintático entende o significado de
`REDEFINES`.

### `invalidos/05-value-nao-suportado.cob` — erro de estrutura

```
erro de estrutura [linha 5, coluna 22]: cláusula 'VALUE' não é suportada nesta etapa: as variáveis são apenas declaradas, sem inicialização (decisão D5 da especificação)
```

`VALUE` é uma palavra reservada legítima de COBOL — o léxico a reconhece
sem problema (`UnsupportedReserved`). O erro é proposital e de estrutura:
o enunciado desta etapa exclui inicialização (decisão D5).

### `invalidos/06-nivel-77-com-subordinado.cob` — erro de estrutura

```
erro de estrutura [linha 6, coluna 1]: nível 05 de 'FILHO' precisa estar subordinado a um item de nível menor, e não há nenhum aberto nesse ponto
```

Nível `77` nunca pode ter subordinados (seção 6): como `CONTADOR` não é
empilhado, o `05` seguinte não encontra nenhum grupo aberto para se
subordinar.

## Cobertura

Seis exemplos inválidos cobrem os dois tipos de erro que o enunciado pede
(léxico e de estrutura), incluindo dois casos onde as duas camadas reagem
em conjunto ao mesmo problema (01 e 03). Quatro exemplos válidos cobrem o
item isolado, o grupo simples, nomes começando por dígito e `REDEFINES`
(sozinho e dentro de um programa maior). `tests/exemplos.rs` automatiza
essa verificação e falha se algum `.cob` novo for adicionado a `exemplos/`
sem um caso correspondente.
