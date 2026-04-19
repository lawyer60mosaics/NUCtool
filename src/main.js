const { listen } = window.__TAURI__.event

const SCENARIOS = {
  silent: {
    mode: 'silent',
    label: '静音',
    tags: ['噪音倾向: 低', '散热倾向: 中'],
    ramp: 2,
    hysteresis: 8,
    cpuCap: 35,
    gpuCap: 30,
    cpuCurve: [20, 20, 22, 24, 27, 30, 32, 34, 35, 35, 35],
    gpuCurve: [18, 18, 20, 22, 24, 26, 28, 29, 30, 30, 30],
  },
  office: {
    mode: 'office',
    label: '办公',
    tags: ['噪音倾向: 中低', '散热倾向: 中高'],
    ramp: 3,
    hysteresis: 5,
    cpuCap: 55,
    gpuCap: 45,
    cpuCurve: [20, 22, 24, 28, 32, 38, 44, 49, 53, 55, 55],
    gpuCurve: [18, 20, 22, 26, 30, 34, 38, 41, 44, 45, 45],
  },
  gaming: {
    mode: 'gaming',
    label: '游戏',
    tags: ['噪音倾向: 中高', '散热倾向: 高'],
    ramp: 5,
    hysteresis: 3,
    cpuCap: 80,
    gpuCap: 90,
    cpuCurve: [25, 28, 33, 40, 48, 56, 64, 71, 76, 79, 80],
    gpuCurve: [28, 32, 40, 50, 60, 70, 78, 84, 88, 90, 90],
  },
  performance: {
    mode: 'performance',
    label: '性能',
    tags: ['噪音倾向: 高', '散热倾向: 极高'],
    ramp: 8,
    hysteresis: 2,
    cpuCap: 100,
    gpuCap: 100,
    cpuCurve: [35, 40, 48, 58, 68, 76, 84, 90, 95, 98, 100],
    gpuCurve: [35, 42, 52, 62, 72, 82, 90, 95, 98, 100, 100],
  },
}

const CURVE_TEMPS = [35, 40, 45, 50, 55, 60, 65, 70, 75, 80, 85]
const HISTORY_POINTS = 21
const MONITOR_MODE_STORAGE_KEY = 'nuctool.monitorModeOnStartup'

const state = {
  running: false,
  selectedScenario: 'office',
  trendChart: null,
  modePoller: null,
  monitorMode: false,
}

function byId(id) {
  return document.getElementById(id)
}

function clamp(v, min, max) {
  const n = Number(v)
  if (!Number.isFinite(n)) return min
  return Math.max(min, Math.min(max, n))
}

function setText(id, text) {
  const el = byId(id)
  if (el) el.textContent = text
}

function setDot(id, dotState) {
  const el = byId(id)
  if (el) el.setAttribute('data-state', dotState)
}

function showToast(message) {
  window.alert(message)
}

function parseMonitorModeFromQuery() {
  const params = new URLSearchParams(window.location.search)
  const raw = params.get('monitor')
  if (!raw) return null

  const normalized = raw.trim().toLowerCase()
  if (['1', 'true', 'on', 'yes', 'full', 'fullscreen'].includes(normalized)) {
    return true
  }
  if (['0', 'false', 'off', 'no'].includes(normalized)) {
    return false
  }
  return null
}

function readMonitorModePreference() {
  const fromQuery = parseMonitorModeFromQuery()
  if (fromQuery !== null) {
    return fromQuery
  }
  try {
    const stored = localStorage.getItem(MONITOR_MODE_STORAGE_KEY)
    if (stored === '1') return true
    if (stored === '0') return false
  } catch {
    // ignore storage failure
  }
  return false
}

function persistMonitorModePreference(enabled) {
  try {
    localStorage.setItem(MONITOR_MODE_STORAGE_KEY, enabled ? '1' : '0')
  } catch {
    // ignore storage failure
  }
}

function monitorModeLabel() {
  return state.monitorMode ? '退出大屏监控' : '进入大屏监控'
}

function setMonitorModeUI(enabled, options = {}) {
  state.monitorMode = enabled
  document.body.classList.toggle('monitor-mode', enabled)
  setText('toggleMonitorModeButton', monitorModeLabel())
  if (options.persist === true) {
    persistMonitorModePreference(enabled)
  }
}

function setRunningUI(running) {
  state.running = running
  setText('btnText', running ? '停止控制' : '启动控制')
  setText('fanStatusText', running ? '运行中' : '待机')
  setDot('fanStatusDot', running ? 'active' : 'idle')
  if (!running) {
    setText('runtimeModeText', `模式: ${SCENARIOS[state.selectedScenario].label}`)
    setDot('modeStatusDot', 'idle')
  }
}

function updateAlert(cpuTemp, gpuTemp) {
  const strip = byId('alertStrip')
  const liveStrip = byId('liveStrip')
  const liveStripText = byId('liveStripText')
  if (!strip) return

  if (cpuTemp >= 90 || gpuTemp >= 90) {
    strip.classList.add('visible')
    strip.textContent = `过温警告：CPU ${cpuTemp}°C / GPU ${gpuTemp}°C，建议切换到“性能场景”。`
    setDot('modeStatusDot', 'danger')
    if (liveStrip && liveStripText) {
      liveStrip.className = 'live-strip hot'
      liveStripText.textContent = '状态色带：高危，温度已进入保护区间，建议立即切换更强散热场景。'
    }
    return
  }

  if (cpuTemp >= 80 || gpuTemp >= 80) {
    strip.classList.add('visible')
    strip.textContent = `高温提醒：CPU ${cpuTemp}°C / GPU ${gpuTemp}°C，建议切换到“游戏场景”或“性能场景”。`
    setDot('modeStatusDot', 'warn')
    if (liveStrip && liveStripText) {
      liveStrip.className = 'live-strip warm'
      liveStripText.textContent = '状态色带：升温，建议关注负载并提前切换到更高散热倾向场景。'
    }
    return
  }

  strip.classList.remove('visible')
  strip.textContent = ''
  if (liveStrip && liveStripText) {
    liveStrip.className = 'live-strip safe'
    liveStripText.textContent = '状态色带：安全，风扇策略处于稳定区间。'
  }
  if (state.running) {
    setDot('modeStatusDot', 'active')
  }
}

function makeTrendChart() {
  const canvas = byId('trendChart')
  if (!canvas) return null

  return new Chart(canvas.getContext('2d'), {
    type: 'line',
    data: {
      labels: Array.from({ length: HISTORY_POINTS }, (_, i) => `${(HISTORY_POINTS - 1 - i) * 3}s`),
      datasets: [
        {
          label: 'CPU 温度',
          data: Array(HISTORY_POINTS).fill(0),
          borderColor: '#d56b41',
          backgroundColor: 'rgba(213, 107, 65, 0.12)',
          borderWidth: 2,
          pointRadius: 0,
          tension: 0.3,
          fill: true,
          yAxisID: 'temp',
        },
        {
          label: 'GPU 温度',
          data: Array(HISTORY_POINTS).fill(0),
          borderColor: '#a24f3a',
          backgroundColor: 'rgba(162, 79, 58, 0.1)',
          borderWidth: 2,
          pointRadius: 0,
          tension: 0.3,
          fill: true,
          yAxisID: 'temp',
        },
        {
          label: 'CPU 风扇 RPM',
          data: Array(HISTORY_POINTS).fill(0),
          borderColor: '#2b75cf',
          backgroundColor: 'rgba(43, 117, 207, 0.1)',
          borderWidth: 2,
          pointRadius: 0,
          tension: 0.3,
          fill: false,
          yAxisID: 'rpm',
        },
        {
          label: 'GPU 风扇 RPM',
          data: Array(HISTORY_POINTS).fill(0),
          borderColor: '#18a29e',
          backgroundColor: 'rgba(24, 162, 158, 0.1)',
          borderWidth: 2,
          pointRadius: 0,
          tension: 0.3,
          fill: false,
          yAxisID: 'rpm',
        },
      ],
    },
    options: {
      responsive: true,
      maintainAspectRatio: false,
      animation: false,
      plugins: {
        legend: {
          labels: {
            color: '#36536f',
            font: { size: 11 },
          },
        },
      },
      scales: {
        x: {
          ticks: {
            color: '#4d667f',
            autoSkip: true,
            maxTicksLimit: 8,
            maxRotation: 0,
          },
          grid: {
            color: 'rgba(29, 45, 64, 0.1)',
          },
        },
        temp: {
          type: 'linear',
          position: 'left',
          min: 0,
          max: 110,
          title: {
            display: true,
            text: '温度 (°C)',
            color: '#4d667f',
            font: { size: 11 },
          },
          ticks: { color: '#4d667f' },
          grid: { color: 'rgba(29, 45, 64, 0.1)' },
        },
        rpm: {
          type: 'linear',
          position: 'right',
          min: 0,
          max: 7000,
          title: {
            display: true,
            text: '风扇 (RPM)',
            color: '#4d667f',
            font: { size: 11 },
          },
          ticks: { color: '#4d667f' },
          grid: {
            drawOnChartArea: false,
          },
        },
      },
    },
  })
}

function pushTrend(cpuTemp, gpuTemp, cpuFan, gpuFan) {
  if (!state.trendChart) return

  const values = [cpuTemp, gpuTemp, cpuFan, gpuFan]
  state.trendChart.data.datasets.forEach((dataset, i) => {
    dataset.data.push(values[i])
    dataset.data.shift()
  })
  state.trendChart.update('none')
}

function scenarioCurvePoints(name, side) {
  const s = SCENARIOS[name] || SCENARIOS.office
  const source = side === 'gpu' ? s.gpuCurve : s.cpuCurve
  return CURVE_TEMPS.map((temperature, i) => ({
    temperature,
    speed: clamp(source[i], 0, 100),
  }))
}

function buildPayload() {
  const s = SCENARIOS[state.selectedScenario] || SCENARIOS.office
  return {
    left_fan: scenarioCurvePoints(state.selectedScenario, 'cpu'),
    right_fan: scenarioCurvePoints(state.selectedScenario, 'gpu'),
    control: {
      mode: s.mode,
      strategy: 'independent',
      preset: s.mode,
      control_mode: 'curve',
      ramp_up_step: s.ramp,
      ramp_down_step: s.ramp,
      min_speed: 20,
      zero_rpm_enabled: false,
      zero_rpm_threshold: 45,
      constant_speed_enabled: false,
      constant_speed: 60,
      cpu_hysteresis_bandwidth: s.hysteresis,
      gpu_hysteresis_bandwidth: s.hysteresis,
      cpu_fan_max_percent: s.cpuCap,
      gpu_fan_max_percent: s.gpuCap,
      gpu_linkage_enabled: true,
      gpu_linkage_threshold: 83,
      gpu_linkage_boost: 10,
    },
    alerts: {
      cpu: {
        threshold: 90,
        actions: {
          popup: true,
          sound: false,
          log: true,
          force_shutdown: false,
          confirm_times: 2,
        },
      },
      gpu: {
        threshold: 90,
        actions: {
          popup: true,
          sound: false,
          log: true,
          force_shutdown: false,
          confirm_times: 2,
        },
      },
      recover_delta: 3,
    },
    monitor: {
      sample_interval_ms: 1000,
      log_enabled: true,
    },
  }
}

async function applyScenarioRuntime() {
  const payload = buildPayload()
  setText('runtimeModeText', `模式: ${SCENARIOS[state.selectedScenario].label}`)
  setDot('modeStatusDot', state.running ? 'active' : 'warn')

  if (state.running) {
    await window.__TAURI__.core.invoke('start_fan_control', { fanData: payload })
  }
}

function updateScenarioButtons() {
  document.querySelectorAll('.scene-btn').forEach((btn) => {
    const active = btn.getAttribute('data-scenario') === state.selectedScenario
    btn.classList.toggle('active', active)
  })
}

async function onScenarioSelected(scenario) {
  if (!SCENARIOS[scenario]) return
  state.selectedScenario = scenario
  updateScenarioButtons()
  await applyScenarioRuntime()
}

function patchFromConfig(fanData) {
  if (!fanData || !fanData.control) return
  const mode = String(fanData.control.mode || fanData.control.preset || 'office')
  if (SCENARIOS[mode]) {
    state.selectedScenario = mode
    updateScenarioButtons()
    setText('runtimeModeText', `模式: ${SCENARIOS[mode].label}`)
  }
}

async function saveConfig() {
  const payload = buildPayload()
  await window.__TAURI__.core.invoke('save_fan_config', { fanData: payload })
  if (state.running) {
    await window.__TAURI__.core.invoke('start_fan_control', { fanData: payload })
  }
}

async function loadConfig() {
  const fanData = await window.__TAURI__.core.invoke('load_fan_config')
  patchFromConfig(fanData)
}

async function startControl() {
  const payload = buildPayload()
  await window.__TAURI__.core.invoke('start_fan_control', { fanData: payload })
  setRunningUI(true)
  setText('runtimeModeText', `模式: ${SCENARIOS[state.selectedScenario].label}`)
  setDot('modeStatusDot', 'active')
}

async function stopControl() {
  await window.__TAURI__.core.invoke('stop_fan_control')
  setRunningUI(false)
}

async function refreshRuntimeMode() {
  try {
    const mode = await window.__TAURI__.core.invoke('get_current_fan_mode')
    if (!mode) return
    const label = SCENARIOS[mode] ? SCENARIOS[mode].label : mode
    setText('runtimeModeText', `模式: ${label}`)
  } catch {
    // ignore polling failures
  }
}

function bindEvents() {
  document.querySelectorAll('.scene-btn').forEach((btn) => {
    btn.addEventListener('click', () => {
      const scenario = String(btn.getAttribute('data-scenario') || 'office')
      onScenarioSelected(scenario).catch((error) => {
        showToast(`切换场景失败\n${error}`)
      })
    })
  })

  byId('startStopButton').addEventListener('click', async () => {
    try {
      if (state.running) {
        await stopControl()
      } else {
        await startControl()
      }
    } catch (error) {
      showToast(`控制操作失败\n${error}`)
    }
  })

  byId('toggleMonitorModeButton').addEventListener('click', () => {
    setMonitorModeUI(!state.monitorMode, { persist: true })
  })

  byId('saveConfigButton').addEventListener('click', async () => {
    try {
      await saveConfig()
      showToast('场景已保存')
    } catch (error) {
      showToast(`保存失败\n${error}`)
    }
  })

  byId('loadConfigButton').addEventListener('click', async () => {
    try {
      await loadConfig()
      await applyScenarioRuntime()
      showToast('已加载上次场景')
    } catch (error) {
      showToast(`加载失败\n${error}`)
    }
  })
}

function bindRealtime() {
  listen('get-fan-speeds', (event) => {
    const payload = event.payload || {}

    const cpuTemp = clamp(payload.left_temp ?? 0, 0, 110)
    const gpuTemp = clamp(payload.right_temp ?? 0, 0, 110)
    const cpuFan = clamp(payload.left_fan_speed ?? 0, 0, 7000)
    const gpuFan = clamp(payload.right_fan_speed ?? 0, 0, 7000)

    setText('cpuTempValue', cpuTemp || '--')
    setText('gpuTempValue', gpuTemp || '--')
    setText('cpuFanValue', cpuFan || '--')
    setText('gpuFanValue', gpuFan || '--')

    updateAlert(cpuTemp, gpuTemp)
    pushTrend(cpuTemp, gpuTemp, cpuFan, gpuFan)
  })
}

async function init() {
  state.trendChart = makeTrendChart()
  updateScenarioButtons()
  bindEvents()
  bindRealtime()
  setMonitorModeUI(readMonitorModePreference(), { persist: false })

  await window.__TAURI__.core.invoke('get_fan_speeds')

  try {
    await loadConfig()
  } catch {
    // keep default scenario
  }

  setRunningUI(false)
  setText('runtimeModeText', `模式: ${SCENARIOS[state.selectedScenario].label}`)

  if (state.modePoller) {
    clearInterval(state.modePoller)
  }
  state.modePoller = setInterval(() => {
    refreshRuntimeMode()
  }, 2500)
}

document.addEventListener('DOMContentLoaded', () => {
  init().catch((error) => {
    showToast(`页面初始化失败\n${error}`)
  })
})
