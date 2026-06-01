import shell from './shell.json'

export default {
  ...shell.common,
  app_title: '虚拟显示器管理器',
  tab_displays: '显示器',
  tab_settings: '设置',
  retry: '重试',
  installing_driver: '正在安装驱动...',
  remove: '移除',
  save: '保存配置',
  saving: '保存中...',
  add: '添加',
  theme_system: '跟随系统',
  theme_light: '浅色',
  theme_dark: '深色',
  connecting: '正在连接原生服务...',
  connection_failed: '无法连接原生服务。',
  operation_failed: '操作失败',
  settings_update_failed: '更新设置失败',
  errors: {
    driver_not_installed: '驱动未安装',
    driver_disabled: '驱动已禁用',
    driver_restart_required: '驱动需要重启',
    driver_error: '驱动发生错误',
    limit_exceeded: '已超出显示器数量限制',
    display_not_found: '未找到显示器',
    unsupported_mode: '不支持的显示模式',
    admin_cancelled: 'UAC 已取消 — 操作未更改',
    driver_installer_missing: '未找到随应用附带的驱动安装包',
    driver_uninstall_failed: '卸载虚拟显示驱动失败',
    driver_uninstall_not_installed: '未检测到已安装的虚拟显示驱动，无需卸载',
    dotnet_runtime_missing: '缺少 Microsoft .NET 8 Runtime (x64)，无法启动随应用附带的原生服务',
    native_host_unavailable: '原生服务不可用',
    config_apply_timeout: '配置已提交但尚未确认生效 — 变更应会稍后生效'
  }
}
