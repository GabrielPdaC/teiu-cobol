*> Programa válido: REDEFINES (decisão D24). CPF e CNPJ dividem o mesmo
*> espaço de CPF-CNPJ, em vez de cada um ocupar um espaço próprio — é assim
*> que um campo de documento de tamanho variável costuma ser modelado em
*> COBOL de verdade.
DATA DIVISION.
WORKING-STORAGE SECTION.
01 PESSOA.
   05 CPF-CNPJ PIC X(14).
   05 CPF       REDEFINES CPF-CNPJ PIC 9(11).
   05 CNPJ      REDEFINES CPF-CNPJ PIC 9(14).
