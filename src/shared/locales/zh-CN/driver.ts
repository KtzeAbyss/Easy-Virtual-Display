export default {
  title: '驱动',
  status: {
    ok: '驱动正常',
    not_installed: '未安装',
    disabled: '已禁用',
    disabled_service: '服务已禁用',
    restart_required: '需要重启',
    inaccessible: '无法访问',
    driver_error: '驱动错误',
    unknown_problem: '未知问题',
    unknown: '未知'
  },
  labels: {
    status: '状态',
    driver_version: '驱动版本',
    max_displays: '显示器上限',
    active_displays: '已启用的显示器',
    parent_gpu: '渲染显卡'
  },
  gpu_options: {
    auto: '自动',
    nvidia: 'NVIDIA',
    amd: 'AMD'
  },
  infobar: {
    not_installed: {
      title: '未安装驱动',
      message: '安装后即可创建虚拟显示器。'
    },
    restart_required: {
      title: '需要重启电脑',
      message: '驱动变更将在重启后生效。'
    },
    disabled: {
      title: '驱动已被禁用',
      message: '请在设备管理器中启用「Parsec Virtual Display Adapter」。'
    },
    unavailable: {
      title: '驱动不可用',
      message: '显示器操作已暂停，重新安装驱动可能会解决问题。'
    }
  },
  open_driver_page: '查看驱动',
  status_title: '状态',
  modes_title: '自定义模式',
  custom_resolutions: '自定义分辨率',
  custom_modes_hint: '最多 5 个额外模式，所有虚拟显示器都可使用。',
  no_custom_modes: '暂无自定义模式',
  add_mode: '添加模式',
  add_custom_mode: '自定义…',
  invalid_values: '请输入正整数',
  placeholder_width: '宽度',
  placeholder_height: '高度',
  placeholder_hz: 'Hz',
  gpu_selection_desc: '用哪块显卡渲染虚拟显示器',
  uac_note: '应用更改需要管理员授权。',
  apply: '应用',
  applying: '正在应用…',
  discard: '放弃更改',
  config_saved: '配置已应用',
  maintenance_title: '维护',
  uninstall: '卸载驱动',
  uninstall_desc: '从本机移除 Parsec 虚拟显示驱动，之后可随时重新安装。',
  uninstall_action: '卸载',
  uninstalling: '正在卸载…',
  uninstall_confirm_title: '卸载虚拟显示驱动？',
  uninstall_confirm_body: '所有虚拟显示器都会被移除，驱动将从本机卸载。此操作需要管理员授权。',
  uninstall_success: '虚拟显示驱动已卸载'
}
