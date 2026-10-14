Substituto em Rust do Rise Mode Temp CPU Driver R2.2: envia a temperatura da CPU para o display do water cooler Rise Mode (HID `AA88:8666`).

## Downloads

| Arquivo | Para quem |
|---|---|
| `risemode-temp-v{{VERSION}}-windows-x64.zip` | **Recomendado.** Programa + scripts de instalar/desinstalar + documentação. |
| `risemode-temp.exe` | Só o executável, para quem quer testar ou instalar à mão. |
| `SHA256SUMS.txt` | Hashes SHA-256 de todos os arquivos. |

## Como instalar (zip)

1. Baixe o `.zip` e, **antes de extrair**, clique com o botão direito → **Propriedades** → marque **Desbloquear**.
2. Extraia e clique com o botão direito em `instalar.ps1` → **Executar com o PowerShell** (aceite o UAC).

Para atualizar uma versão já instalada, é só rodar o `instalar.ps1` da versão nova.

## Só o .exe

Num PowerShell **como administrador**, na pasta do arquivo:

- `.\risemode-temp.exe --console`: testa, mostrando a temperatura lida no terminal.
- `.\risemode-temp.exe --install`: agenda para iniciar no logon a partir da pasta onde o `.exe` está (então deixe-o num lugar fixo).
- `.\risemode-temp.exe --uninstall`: remove a tarefa agendada.
- `.\risemode-temp.exe --help`: lista todas as opções.

Feche o driver original da Rise Mode antes, porque ele segura o display só para ele.

Requisitos: CPU AMD Ryzen (Zen a Zen 5), [PawnIO](https://pawnio.eu) instalado e o display conectado por USB. Detalhes no README.

## Verificação

O `.exe` não é assinado digitalmente, então o SmartScreen pode avisar. Ele foi compilado pelo GitHub Actions a partir
deste commit; para conferir a origem (precisa do [GitHub CLI](https://cli.github.com)):

```
gh attestation verify risemode-temp.exe --repo Jaimedsf/risemode-temp
```

SHA-256 (confira com `Get-FileHash <arquivo>`):

```
{{SHA256}}
```

Projeto não oficial, sem afiliação com a Rise Mode.
