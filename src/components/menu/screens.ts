export type MenuScreen =
  'main' | 'worlds' | 'create' | 'delete' | 'settings' | 'graphics' | 'controls'

export const TITLES: Record<MenuScreen, string> = {
  main: 'MineRust',
  worlds: 'Выбор мира',
  create: 'Создание мира',
  delete: 'Удаление мира',
  settings: 'Настройки',
  graphics: 'Графика',
  controls: 'Управление',
}

export function backTarget(screen: MenuScreen, inWorld: boolean): MenuScreen {
  switch (screen) {
    case 'create':
    case 'delete':
      return 'worlds'
    case 'graphics':
      return 'settings'
    case 'controls':
      return inWorld ? 'main' : 'settings'
    default:
      return 'main'
  }
}
