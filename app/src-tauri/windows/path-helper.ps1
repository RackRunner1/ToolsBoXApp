param(
    [Parameter(Mandatory = $true)][ValidateSet('add', 'remove')][string]$Action,
    [Parameter(Mandatory = $true)][string]$Dir
)

$target = $Dir.TrimEnd('\')
$current = [Environment]::GetEnvironmentVariable('Path', 'User')
$entries = @()
if ($current) {
    $entries = @($current -split ';' | Where-Object { $_ -ne '' -and $_.TrimEnd('\') -ine $target })
}
if ($Action -eq 'add') {
    $entries += $target
}
[Environment]::SetEnvironmentVariable('Path', ($entries -join ';'), 'User')
