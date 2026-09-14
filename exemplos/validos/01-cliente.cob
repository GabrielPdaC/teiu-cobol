*> Programa válido: item isolado e item de grupo com subordinados.
DATA DIVISION.
WORKING-STORAGE SECTION.
77 CONTADOR       PIC S9(4).
01 CLIENTE.
   05 CNPJ        PIC 9(14).
   05 CPF REDEFINES CNPJ PIC 9(11).
   05 NOME        PIC X(30).
   05 SALDO       PIC S9(7)V99.
