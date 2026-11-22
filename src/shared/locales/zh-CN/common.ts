import shell from './shell.json'

export default {
  ...shell.common,
  app_title: 'Easy Virtual Display',
  nav_label: '主导航',
  nav: {
    displays: '显示器',
    driver: '驱动',
    settings: '设置',
    about: '关于'
  },
  window: {
    minimize: '最小化',
    maximize: '最大化',
    restore: '向下还原',
    close: '关闭'
  },
  retry: '重试',
  installing_driver: '正在安装驱动…',
  more_actions: '更多操作',
  undo: '撤销',
  theme_system: '跟随系统',
  theme_light: '浅色',
  theme_dark: '深色',
  connecting: '正在启动后台服务…',
  connection_failed: '无法连接后台服务。',
  startup_failed_title: 'Easy Virtual Display 无法启动',
  operation_failed: '操作失败',
  settings_update_failed: '无法更改此设置',
  about: {
    description: '在 Windows 上创建和管理虚拟显示器。',
    version: '版本 {{version}}',
    driver_credit: '虚拟显示能力由 Parsec Virtual Display Driver 提供。',
    links_title: '链接',
    repository: '项目主页',
    issues: '反馈问题',
    releases: '更新日志',
    shortcuts_title: '键盘快捷键',
    shortcut_add_display: '添加显示器',
    shortcut_settings: '打开设置'
  },
  errors: {
    driver_not_installed: '驱动未安装',
    driver_disabled: '驱动已被禁用',
    driver_restart_required: '需要重启电脑以完成驱动变更',
    driver_error: '驱动报告了错误',
    limit_exceeded: '已达到显示器数量上限',
    display_not_found: '找不到该显示器',
    unsupported_mode: '不支持的显示模式',
    admin_cancelled: '已取消管理员授权，未做任何更改。',
    driver_installer_missing: '找不到随应用附带的驱动安装包',
    driver_uninstall_failed: '无法卸载虚拟显示驱动',
    driver_uninstall_not_installed: '未安装虚拟显示驱动',
    dotnet_runtime_missing: '缺少 Microsoft .NET 8 Runtime (x64)，无法启动后台服务',
    native_host_unavailable: '后台服务不可用',
    config_apply_timeout: '配置已提交，但尚未确认生效，稍后应会生效。',
    viewer_unsupported: '仅 Windows 支持预览',
    viewer_start_failed: '无法打开预览窗口',
    launch_on_login_failed: '无法更改开机启动设置',
    settings_save_failed: '无法保存设置',
    invalid_request: '请求无效',
    needs_elevation: '需要管理员授权'
  }
}
