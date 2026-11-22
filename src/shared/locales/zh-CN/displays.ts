export default {
  title: '显示器',
  count_of_max: '{{count}} / {{max}}',
  add_display: '添加显示器',
  limit_reached: '驱动最多支持 {{max}} 个显示器',
  remove: '移除',
  remove_all: '全部移除',
  remove_all_confirm_title: '移除全部虚拟显示器？',
  remove_all_confirm_body: '将移除 {{count}} 个虚拟显示器，其上的窗口会移到其他显示器。',
  open_display_settings: '系统显示设置',
  state_active: '已启用',
  state_inactive: '未启用',
  device_name: '设备：{{name}}',
  unsupported_mode: '模式不受支持',
  resolution: '分辨率',
  refresh_rate: '刷新率',
  orientation: '方向',
  orientations: {
    landscape: '横向',
    portrait: '纵向',
    landscape_flipped: '横向（翻转）',
    portrait_flipped: '纵向（翻转）'
  },
  hz: '{{value}} Hz',
  mode_changed: '显示模式已更改',
  empty_title: '还没有虚拟显示器',
  empty_description: '点击「添加显示器」或按 Ctrl+N 创建一个。',
  empty_no_driver_title: '安装驱动后即可开始使用',
  empty_no_driver_description: '虚拟显示器由 Parsec 虚拟显示驱动提供，只需安装一次。',
  open_viewer: '预览',
  close_viewer: '停止预览',
  viewer_hint:
    '在窗口中镜像该显示器的画面。点击窗口内部即可接管鼠标键盘，按标题栏提示的快捷键可释放。若快捷键被该显示器上的程序拦截（如全屏远程桌面），可按 Ctrl+Alt+Del 再按 Esc 释放。'
}
