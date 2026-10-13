param([string]$Path, [string]$TypeFilter = '.*', [switch]$ListOnly)
# Mini IL disassembler via reflection (no code from the target is executed).
$ops = @{}
foreach ($fi in [Reflection.Emit.OpCodes].GetFields('Public,Static')) {
    $op = $fi.GetValue($null); $ops[[uint16]([int]$op.Value -band 0xFFFF)] = $op
}
$dir = Split-Path $Path
[AppDomain]::CurrentDomain.add_AssemblyResolve({ param($s, $e)
    $n = (New-Object Reflection.AssemblyName $e.Name).Name
    $p = Join-Path $dir "$n.dll"; if (Test-Path $p) { [Reflection.Assembly]::LoadFile($p) } })
$asm = [Reflection.Assembly]::LoadFile($Path)
$mod = $asm.ManifestModule
try { $types = $asm.GetTypes() } catch [Reflection.ReflectionTypeLoadException] { $types = $_.Exception.Types | Where-Object { $_ } }
$bf = [Reflection.BindingFlags]'Public,NonPublic,Instance,Static,DeclaredOnly'
foreach ($t in $types) {
    if ($t.FullName -notmatch $TypeFilter) { continue }
    "##### TYPE $($t.FullName) : $($t.BaseType)"
    foreach ($f in $t.GetFields($bf)) { "  field $($f.FieldType.Name) $($f.Name)" }
    $members = @($t.GetConstructors($bf)) + @($t.GetMethods($bf))
    foreach ($m in $members) {
        $sig = ($m.GetParameters() | ForEach-Object { "$($_.ParameterType.Name) $($_.Name)" }) -join ', '
        "  ---- METHOD $($m.Name)($sig)"
        if ($ListOnly) { continue }
        $body = $m.GetMethodBody(); if (-not $body) { continue }
        $il = $body.GetILAsByteArray(); $i = 0
        while ($i -lt $il.Length) {
            $start = $i; $code = [uint16]$il[$i++]
            if ($code -eq 0xFE) { $code = [uint16](0xFE00 -bor $il[$i++]) }
            $op = $ops[$code]; if (-not $op) { "    IL_{0:X4}: ??{1:X}" -f $start, $code; continue }
            $arg = ''
            switch ($op.OperandType) {
                'InlineNone' { }
                'ShortInlineBrTarget' { $arg = 'IL_{0:X4}' -f ($i + 1 + [sbyte][int](($il[$i] -bxor 0x80) - 0x80)); $i += 1 }
                'ShortInlineI' { $arg = [sbyte][int](($il[$i] -bxor 0x80) - 0x80); $i += 1 }
                'ShortInlineVar' { $arg = $il[$i]; $i += 1 }
                'InlineVar' { $arg = [BitConverter]::ToUInt16($il, $i); $i += 2 }
                'InlineBrTarget' { $arg = 'IL_{0:X4}' -f ($i + 4 + [BitConverter]::ToInt32($il, $i)); $i += 4 }
                'InlineI' { $arg = [BitConverter]::ToInt32($il, $i); $i += 4 }
                'InlineI8' { $arg = [BitConverter]::ToInt64($il, $i); $i += 8 }
                'ShortInlineR' { $arg = [BitConverter]::ToSingle($il, $i); $i += 4 }
                'InlineR' { $arg = [BitConverter]::ToDouble($il, $i); $i += 8 }
                'InlineSwitch' { $n = [BitConverter]::ToInt32($il, $i); $i += 4 + 4 * $n; $arg = "switch($n)" }
                'InlineString' { $tok = [BitConverter]::ToInt32($il, $i); $i += 4; $arg = '"' + $mod.ResolveString($tok) + '"' }
                default {
                    $tok = [BitConverter]::ToInt32($il, $i); $i += 4
                    try { $r = $mod.ResolveMember($tok, $t.GetGenericArguments(), $(if ($m.IsGenericMethod) { $m.GetGenericArguments() } else { $null }))
                          $arg = "$($r.DeclaringType.Name)::$($r.Name)"; if ($r -is [Type]) { $arg = $r.FullName } }
                    catch { $arg = 'tok_{0:X8}' -f $tok }
                }
            }
            "    IL_{0:X4}: {1} {2}" -f $start, $op.Name, $arg
        }
    }
}
