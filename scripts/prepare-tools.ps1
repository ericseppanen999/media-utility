param([switch]$RefreshLock)
$ErrorActionPreference='Stop'
$root=Split-Path $PSScriptRoot -Parent
$bin=Join-Path $root 'src-tauri\binaries'
$cache=Join-Path $root 'tools-cache'
$lockPath=Join-Path $PSScriptRoot 'dependencies.lock.json'
New-Item -ItemType Directory -Force -Path $bin,$cache | Out-Null
$headers=@{'User-Agent'='Media-Utility-build'}
function Fetch([string]$url,[string]$out){
 Write-Host "Downloading $url"
 Invoke-WebRequest -Uri $url -OutFile $out -Headers $headers -UseBasicParsing
}
if($RefreshLock -or !(Test-Path -LiteralPath $lockPath)){
 $yt=Invoke-RestMethod 'https://api.github.com/repos/yt-dlp/yt-dlp/releases/latest' -Headers $headers
 $deno=Invoke-RestMethod 'https://api.github.com/repos/denoland/deno/releases/latest' -Headers $headers
 $version=(Invoke-RestMethod 'https://www.gyan.dev/ffmpeg/builds/release-version').Trim()
 $ffUrl="https://www.gyan.dev/ffmpeg/builds/packages/ffmpeg-$version-essentials_build.zip"
 $ffHash=((Invoke-RestMethod "$ffUrl.sha256") -split '\s+')[0].ToLower()
 $ytAsset=$yt.assets | Where-Object name -eq 'yt-dlp.exe'
 $denoAsset=$deno.assets | Where-Object name -eq 'deno-x86_64-pc-windows-msvc.zip'
 $entries=@(
  @{name='yt-dlp';version=$yt.tag_name;url=$ytAsset.browser_download_url;sha256=$ytAsset.digest.Replace('sha256:','');archive='yt-dlp.exe'},
  @{name='deno';version=$deno.tag_name;url=$denoAsset.browser_download_url;sha256=$denoAsset.digest.Replace('sha256:','');archive='deno.zip'},
  @{name='ffmpeg';version=$version;url=$ffUrl;sha256=$ffHash;archive='ffmpeg.zip'}
 )
 foreach($item in $entries){if($item.sha256 -notmatch '^[0-9a-f]{64}$'){throw "Missing published checksum for $($item.name)"}}
 @{platform='windows-x86_64';dependencies=$entries} | ConvertTo-Json -Depth 6 | Set-Content -LiteralPath $lockPath -Encoding UTF8
}
$lock=Get-Content -LiteralPath $lockPath -Raw | ConvertFrom-Json
foreach($dep in $lock.dependencies){
 $archive=Join-Path $cache $dep.archive
 if(!(Test-Path -LiteralPath $archive) -or (Get-FileHash -LiteralPath $archive -Algorithm SHA256).Hash.ToLower() -ne $dep.sha256){Fetch $dep.url $archive}
 if((Get-FileHash -LiteralPath $archive -Algorithm SHA256).Hash.ToLower() -ne $dep.sha256){throw "Integrity check failed for $($dep.name)"}
 if($dep.name -eq 'yt-dlp'){Copy-Item -LiteralPath $archive -Destination (Join-Path $bin 'yt-dlp.exe') -Force}
 else {
  $unpack=Join-Path $cache "$($dep.name)-unpacked"
  Expand-Archive -LiteralPath $archive -DestinationPath $unpack -Force
  $names=if($dep.name -eq 'deno'){@('deno.exe')}else{@('ffmpeg.exe','ffprobe.exe')}
  foreach($name in $names){
   $found=Get-ChildItem -LiteralPath $unpack -Filter $name -Recurse | Select-Object -First 1
   if(!$found){throw "Archive is missing $name"}
   Copy-Item -LiteralPath $found.FullName -Destination (Join-Path $bin $name) -Force
  }
  if($dep.name -eq 'ffmpeg'){
   $license=Get-ChildItem -LiteralPath $unpack -Filter 'LICENSE*' -Recurse | Select-Object -First 1
   if($license){Copy-Item -LiteralPath $license.FullName -Destination (Join-Path $bin 'FFmpeg-LICENSE.txt') -Force}
  }
 }
}
Copy-Item -LiteralPath $lockPath -Destination (Join-Path $bin 'dependencies.lock.json') -Force
Write-Host 'Verified tools are ready in src-tauri/binaries.'
