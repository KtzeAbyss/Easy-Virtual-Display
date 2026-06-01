export default {
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
    overview: '概览',
    driver_status: '驱动状态',
    driver_version: '驱动版本',
    max_displays: '最大显示器数',
    active_displays: '活动显示器',
    parent_gpu: '父 GPU'
  },
  gpu_options: {
    auto: '自动',
    nvidia: 'NVIDIA',
    amd: 'AMD'
  },
  warnings: {
    driver_not_ready: '驱动未就绪 — 显示操作不可用。'
  }
}
