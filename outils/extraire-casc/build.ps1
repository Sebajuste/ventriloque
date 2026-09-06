# Bâtit extraire-casc.exe avec cl.exe, sans CMake.
#
# CascLib fournit deux fichiers de compilation unifiée — `sources-c.c` et `sources-cpp.cpp` —
# qui incluent tout le reste. Trois appels à cl.exe suffisent donc, et le CMakeLists du dépôt
# n'a pas à être utilisé : cmake n'est pas installé sur cette machine.
#
# Usage :
#   powershell -File outils\extraire-casc\build.ps1

$ErrorActionPreference = "Stop"
$ici = $PSScriptRoot
$casclib = Join-Path $ici "vendor\CascLib"
$obj = Join-Path $ici "obj"
$exe = Join-Path $ici "extraire-casc.exe"

if (-not (Test-Path (Join-Path $casclib "sources-cpp.cpp"))) {
    throw "CascLib absent. Depuis $ici : git clone --depth 1 https://github.com/ladislav-zezula/CascLib.git vendor\CascLib"
}

# cl.exe n'est pas dans le PATH : c'est vcvars64 qui l'y met, et vswhere qui trouve vcvars64.
$vswhere = "${env:ProgramFiles(x86)}\Microsoft Visual Studio\Installer\vswhere.exe"
if (-not (Test-Path $vswhere)) { throw "vswhere introuvable — Visual Studio n'est pas installé" }
$vs = & $vswhere -latest -products * -requires Microsoft.VisualStudio.Component.VC.Tools.x86.x64 -property installationPath
if (-not $vs) { throw "aucune installation Visual Studio avec les outils C++ (composant « MSVC v143 »)" }
$vcvars = Join-Path $vs "VC\Auxiliary\Build\vcvars64.bat"

if (-not (Test-Path $obj)) { New-Item -ItemType Directory $obj | Out-Null }

# Tout tient dans un seul appel à cmd : l'environnement posé par vcvars64 ne survit pas au
# processus, donc la compilation doit se faire dans la foulée.
$commandes = @(
    "call `"$vcvars`" >nul 2>&1",
    "cd /d `"$obj`"",
    "cl /nologo /c /O2 /MT /EHsc /D_CRT_SECURE_NO_WARNINGS /DCASCLIB_NO_AUTO_LINK_LIBRARY `"$casclib\sources-c.c`"",
    "cl /nologo /c /O2 /MT /EHsc /D_CRT_SECURE_NO_WARNINGS /DCASCLIB_NO_AUTO_LINK_LIBRARY `"$casclib\sources-cpp.cpp`"",
    "cl /nologo /c /O2 /MT /EHsc /D_CRT_SECURE_NO_WARNINGS /DCASCLIB_NO_AUTO_LINK_LIBRARY /I`"$casclib\src`" `"$ici\src\extraire.cpp`"",
    "link /nologo /OUT:`"$exe`" sources-c.obj sources-cpp.obj extraire.obj advapi32.lib ws2_32.lib"
) -join " && "

cmd /c $commandes
if ($LASTEXITCODE -ne 0) { throw "la compilation a échoué" }

$poids = [math]::Round((Get-Item $exe).Length / 1MB, 2)
Write-Output ""
Write-Output "outils\extraire-casc\extraire-casc.exe — $poids Mo"
