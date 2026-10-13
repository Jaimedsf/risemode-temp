# Avisos de terceiros

## PawnIO Modules — `third_party/pawnio-modules/`

- **Arquivos:** `AMDFamily17.bin` (módulo compilado e assinado), `AMDFamily17.p` (código-fonte).
- **Autor:** namazso — Copyright (C) 2025 namazso <admin@namazso.eu>
- **Licença:** GNU Lesser General Public License v2.1 ou posterior (`LGPL-2.1-or-later`). Texto completo em
  [`third_party/pawnio-modules/COPYING`](third_party/pawnio-modules/COPYING).
- **Fonte original:** <https://github.com/namazso/PawnIO.Modules>
- **Origem do binário:** extraído sem modificação do recurso `LibreHardwareMonitor.Resources.PawnIo.AMDFamily17.bin`
  do `LibreHardwareMonitorLib.dll` 0.9.5 (SHA-256
  `CD59598344B54A23178CCF9223239F60A3111BD86FF2C7BEA2C9D1CEF89E095B`). Builds assinados oficiais:
  <https://github.com/namazso/PawnIO.Modules/releases>.

O módulo é embutido no executável apenas como dados (`include_bytes!`) e é executado pela máquina virtual do driver
PawnIO, não pelo programa. Você pode substituí-lo por outra versão do módulo, trocando o arquivo e recompilando.

## PawnIO (driver) e PawnIOLib — não incluídos

O driver `PawnIO.sys` e a `PawnIOLib.dll` (LGPL-2.1, namazso, <https://pawnio.eu>) **não** são distribuídos aqui.
São carregados em tempo de execução a partir da instalação do usuário.

## LibreHardwareMonitor — referência

Nenhum código foi copiado. O endereço do registrador (`0x59800`) e a fórmula de conversão da temperatura foram
obtidos estudando o LibreHardwareMonitor (MPL-2.0, <https://github.com/LibreHardwareMonitor/LibreHardwareMonitor>).

## Rise Mode — marca e protocolo

Projeto **não oficial**, sem afiliação com a Rise Mode. "Rise Mode" é marca do respectivo titular e aparece aqui só
para indicar compatibilidade. Nenhum código, binário ou recurso do "Rise Mode Temp CPU Driver" é distribuído. O
formato do relatório USB foi documentado por engenharia reversa, para interoperabilidade com o hardware do usuário.
