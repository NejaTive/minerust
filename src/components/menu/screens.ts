export type MenuScreen = 'main' | 'worlds' | 'create' | 'delete' | 'settings' | 'controls'

export const TITLES: Record<MenuScreen, string> = {
  main: 'MineRust',
  worlds: 'Выбор мира',
  create: 'Создание мира',
  delete: 'Удаление мира',
  settings: 'Настройки',
  controls: 'Управление',
}

export function backTarget(screen: MenuScreen, inWorld: boolean): MenuScreen {
  switch (screen) {
    case 'create':
    case 'delete':
      return 'worlds'
    case 'controls':
      return inWorld ? 'main' : 'settings'
    default:
      return 'main'
  }
}
