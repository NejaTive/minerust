// Поднимает версию: `node scripts/release.mjs <minor|patch|X.Y.Z>` меняет package.json,
// package-lock.json и engine/Cargo.toml (+ Cargo.lock). CHANGELOG, коммит и тег — скилл `release`.
import { execFileSync } from 'node:child_process'
import { readFileSync, writeFileSync } from 'node:fs'

const readJson = (path) => JSON.parse(readFileSync(path, 'utf8'))
const writeJson = (path, value) => writeFileSync(path, JSON.stringify(value, null, 2) + '\n')

function bump(current, kind) {
  if (/^\d+\.\d+\.\d+$/.test(kind)) return kind
  const [major, minor, patch] = current.split('.').map(Number)
  if (kind === 'minor') return `${major}.${minor + 1}.0`
  if (kind === 'patch') return `${major}.${minor}.${patch + 1}`
  throw new Error('usage: release.mjs <minor|patch|X.Y.Z>')
}

function setVersion(version) {
  const pkg = readJson('package.json')
  pkg.version = version
  writeJson('package.json', pkg)

  const lock = readJson('package-lock.json')
  lock.version = version
  lock.packages[''].version = version
  writeJson('package-lock.json', lock)

  // Первая `version =` в файле — пакет движка, зависимости идут ниже.
  const cargo = readFileSync('engine/Cargo.toml', 'utf8').replace(/^version = ".*"$/m, `version = "${version}"`)
  writeFileSync('engine/Cargo.toml', cargo)
  execFileSync('cargo', ['update', '-p', 'engine', '--offline'], { cwd: 'engine', stdio: 'ignore' })
}

const version = bump(readJson('package.json').version, process.argv[2])
setVersion(version)
console.log(`version ${version}`)
