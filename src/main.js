const { listen } = window.__TAURI__.event;

// 左风扇曲线
const leftFanCtx = document.getElementById('leftFanCurve').getContext('2d');
const leftFanCurve = new Chart(leftFanCtx, {
    type: 'line',
    data: {
        labels: Array.from({length: 15}, (_, i) => (i + 30) + Math.round(i * 4)),  // 30 - 95度
        datasets: [{
            label: 'CPU风扇速度',
            data: Array(66).fill(50),  // 默认风扇速度
            borderColor: 'blue',
            fill: false
        }]
    },
    options: {
        scales: {
            x: {title: {display: true, text: '温度 (°C)'}},
            y: {title: {display: true, text: '风扇速度 (%)'}, min: 0, max: 100}
        },
        plugins: {
            dragData: {
                round: 0,
                onDrag: function (e, datasetIndex, index, value) {
                    console.log(`Left Fan - Temperature: ${leftFanCurve.data.labels[index]}, Speed: ${value}%`);
                }
            },
        },
        animations: {
            tension: {
                duration: 1000,
                easing: 'linear',
                from: 1,
                to: 0,
                loop: false
            }
        },
        cubicInterpolationMode: 'monotone'
    }
});

// 右风扇曲线
const rightFanCtx = document.getElementById('rightFanCurve').getContext('2d');
const rightFanCurve = new Chart(rightFanCtx, {
    type: 'line',
    data: {
        labels: Array.from({length: 15}, (_, i) => (i + 30) + Math.round(i * 4)),
        datasets: [{
            label: 'GPU风扇速度',
            data: Array(15).fill(50),
            borderColor: 'rgba(16, 185, 129, 1)',
            backgroundColor: 'rgba(16, 185, 129, 0.1)',
            borderWidth: 3,
            pointRadius: 6,
            pointHoverRadius: 8,
            pointBackgroundColor: 'rgba(16, 185, 129, 1)',
            pointBorderColor: '#fff',
            pointBorderWidth: 2,
            tension: 0.4,
            fill: true
        }]
    },
    options: {
        responsive: true,
        maintainAspectRatio: false,
        animation: false,
        plugins: {
            legend: {
                display: true,
                labels: {
                    color: '#e5e5e5',
                    font: { size: 12, weight: '600' },
                    padding: 15
                }
            },
            dragData: {
                round: 0,
                onDrag: function (e, datasetIndex, index, value) {
                    console.log(`GPU Fan - Temp: ${rightFanCurve.data.labels[index]}°C, Speed: ${value}%`);
                }
            }
        },
        scales: {
            x: {
                title: {
                    display: true,
                    text: '温度 (°C)',
                    color: 'rgba(255, 255, 255, 0.8)',
                    font: { size: 13, weight: '600' }
                },
                ticks: {
                    color: 'rgba(255, 255, 255, 0.7)',
                    font: { size: 11 }
                },
                grid: {
                    color: 'rgba(255, 255, 255, 0.1)',
                    borderColor: 'rgba(255, 255, 255, 0.2)'
                }
            },
            y: {
                title: {
                    display: true,
                    text: '风扇速度 (%)',
                    color: 'rgba(255, 255, 255, 0.8)',
                    font: { size: 13, weight: '600' }
                },
                min: 0,
                max: 100,
                ticks: {
                    color: 'rgba(255, 255, 255, 0.7)',
                    font: { size: 11 }
                },
                grid: {
                    color: 'rgba(255, 255, 255, 0.1)',
                    borderColor: 'rgba(255, 255, 255, 0.2)'
                }
            }
        }
    }
});

const leftFanSpeedCtx = document.getElementById('left_fan_speed').getContext('2d');
const rightFanSpeedCtx = document.getElementById('right_fan_speed').getContext('2d');

// 初始化风扇实时转速图表
function initSpeedCharts() {
    const leftFanSpeedChart = new Chart(leftFanSpeedCtx, {
        type: 'line',
        data: {
            labels: Array.from({length: 21}, (_, i) => i * 3),
            datasets: [{
                label: 'CPU风扇转速',
                data: Array(21).fill(0),
                borderColor: 'rgba(59, 130, 246, 1)',
                backgroundColor: 'rgba(59, 130, 246, 0.1)',
                borderWidth: 2,
                pointRadius: 0,
                tension: 0.4,
                fill: true
            }, {
                label: 'GPU风扇转速',
                data: Array(21).fill(0),
                borderColor: 'rgba(16, 185, 129, 1)',
                backgroundColor: 'rgba(16, 185, 129, 0.1)',
                borderWidth: 2,
                pointRadius: 0,
                tension: 0.4,
                fill: true
            }]
        },
        options: {
            responsive: true,
            maintainAspectRatio: false,
            animation: false,
            plugins: {
                legend: {
                    display: true,
                    labels: {
                        color: 'rgba(255, 255, 255, 0.9)',
                        font: { size: 11, weight: '600' },
                        padding: 12
                    }
                }
            },
            scales: {
                x: {
                    title: {
                        display: true,
                        text: '时间 (秒)',
                        color: 'rgba(255, 255, 255, 0.8)',
                        font: { size: 12, weight: '600' }
                    },
                    ticks: {
                        color: 'rgba(255, 255, 255, 0.7)',
                        font: { size: 10 }
                    },
                    grid: {
                        color: 'rgba(255, 255, 255, 0.1)',
                        borderColor: 'rgba(255, 255, 255, 0.2)'
                    }
                },
                y: {
                    title: {
                        display: true,
                        text: '转速 (RPM)',
                        color: 'rgba(255, 255, 255, 0.8)',
                        font: { size: 12, weight: '600' }
                    },
                    min: 0,
                    max: 6000,
                    ticks: {
                        color: 'rgba(255, 255, 255, 0.7)',
                        font: { size: 10 }
                    },
                    grid: {
                        color: 'rgba(255, 255, 255, 0.1)',
                        borderColor: 'rgba(255, 255, 255, 0.2)'
                    }
                }
            }
        },
    });

    const rightFanSpeedChart = new Chart(rightFanSpeedCtx, {
        type: 'line',
        data: {
            labels: Array.from({length: 21}, (_, i) => i * 3),
            datasets: [{
                label: 'CPU温度',
                data: Array(21).fill(0),
                borderColor: 'rgba(239, 68, 68, 1)',
                backgroundColor: 'rgba(239, 68, 68, 0.1)',
                borderWidth: 2,
                pointRadius: 0,
                tension: 0.4,
                fill: true
            }, {
                label: 'GPU温度',
                data: Array(21).fill(0),
                borderColor: 'rgba(245, 158, 11, 1)',
                backgroundColor: 'rgba(245, 158, 11, 0.1)',
                borderWidth: 2,
                pointRadius: 0,
                tension: 0.4,
                fill: true
            }]
        },
        options: {
            responsive: true,
            maintainAspectRatio: false,
            animation: false,
            plugins: {
                legend: {
                    display: true,
                    labels: {
                        color: '#e5e5e5',
                        font: { size: 11, weight: '600' },
                        padding: 12
                    }
                }
            },
            scales: {
                x: {
                    title: {
                        display: true,
                        text: '时间 (秒)',
                        color: '#e5e5e5',
                        font: { size: 12, weight: '600' }
                    },
                    ticks: {
                        color: '#888',
                        font: { size: 10 }
                    },
                    grid: {
                        color: '#2a2a2a',
                        borderColor: '#3a3a3a'
                    }
                },
                y: {
                    title: {
                        display: true,
                        text: '温度 (℃)',
                        color: '#e5e5e5',
                        font: { size: 12, weight: '600' }
                    },
                    min: 0,
                    max: 100,
                    ticks: {
                        color: '#888',
                        font: { size: 10 }
                    },
                    grid: {
                        color: '#2a2a2a',
                        borderColor: '#3a3a3a'
                    }
                }
            }
        }
    });

    return {leftFanSpeedChart, rightFanSpeedChart};
}

// 更新风扇实时转速数据
function updateFanSpeeds(leftFanSpeedChart, rightFanSpeedChart, left_fan_speed, right_fan_speed, left_temp, right_temp) {
    // 数据验证，允许温度达到105度以支持高温场景
    if (left_fan_speed < 0 || right_fan_speed < 0 || left_fan_speed > 7000 || right_fan_speed > 7000 ||
        left_temp < 0 || right_temp < 0 || left_temp > 105 || right_temp > 105) {
        console.warn(`Invalid sensor data: fan_speed(${left_fan_speed}, ${right_fan_speed}), temp(${left_temp}, ${right_temp})`);
        return;
    }

    // 更新实时指标卡
    document.getElementById('cpuTempValue').textContent = left_temp;
    document.getElementById('gpuTempValue').textContent = right_temp;
    document.getElementById('cpuFanValue').textContent = left_fan_speed;
    document.getElementById('gpuFanValue').textContent = right_fan_speed;

    // 温度警告颜色
    const cpuTempCard = document.getElementById('cpuTempValue');
    const gpuTempCard = document.getElementById('gpuTempValue');

    if (left_temp > 85) {
        cpuTempCard.style.color = '#FF5252';
    } else if (left_temp > 70) {
        cpuTempCard.style.color = '#FF9800';
    } else {
        cpuTempCard.style.color = '#4CAF50';
    }

    if (right_temp > 85) {
        gpuTempCard.style.color = '#FF5252';
    } else if (right_temp > 70) {
        gpuTempCard.style.color = '#FF9800';
    } else {
        gpuTempCard.style.color = '#4CAF50';
    }

    leftFanSpeedChart.data.datasets[0].data.push(left_fan_speed);
    leftFanSpeedChart.data.datasets[0].data.shift(); // 移除最早的数据
    leftFanSpeedChart.data.datasets[1].data.push(right_fan_speed);
    leftFanSpeedChart.data.datasets[1].data.shift();
    rightFanSpeedChart.data.datasets[0].data.push(left_temp);
    rightFanSpeedChart.data.datasets[0].data.shift();
    rightFanSpeedChart.data.datasets[1].data.push(right_temp);
    rightFanSpeedChart.data.datasets[1].data.shift();
    // console.log(`Left Fan - Speed: ${leftFanSpeedChart.data.datasets[0].data}, Right Fan - Speed: ${rightFanSpeedChart.data.datasets[0].data}`);
    leftFanSpeedChart.update();
    rightFanSpeedChart.update();
}

async function loadConfigData() {
    try {
        const fanData = await window.__TAURI__.core.invoke('load_fan_config');
        if (fanData) {
            // 更新左风扇曲线数据
            leftFanCurve.data.datasets[0].data = fanData.left_fan.map(point => point.speed);
            leftFanCurve.update();

            // 更新右风扇曲线数据
            rightFanCurve.data.datasets[0].data = fanData.right_fan.map(point => point.speed);
            rightFanCurve.update();

            // 更新状态指示器
            const configDot = document.getElementById('configStatusDot');
            const configText = document.getElementById('configStatusText');
            configDot.className = 'status-dot active';
            configText.textContent = '配置: 已加载';

            console.log('配置文件加载成功');
        }
    } catch (error) {
        console.error('配置文件加载失败:', error);

        // 更新状态指示器为警告
        const configDot = document.getElementById('configStatusDot');
        const configText = document.getElementById('configStatusText');
        configDot.className = 'status-dot warning';
        configText.textContent = '配置: 加载失败';

        alert('配置文件加载失败，将使用默认配置');
    }
}

document.addEventListener('DOMContentLoaded', async () => {
    console.log('🚀 NUCtool 风扇控制系统启动中...');

    const {leftFanSpeedChart, rightFanSpeedChart} = initSpeedCharts();
    console.log('📊 实时监控图表初始化完成');

    const startStopButton = document.getElementById('startStopButton');
    const saveConfigButton = document.getElementById('saveConfigButton');
    const loadConfigButton = document.getElementById('loadConfigButton');
    let isRunning = false;

    // 设置实时数据监听
    async function listen_to_greet() {
        await listen('get-fan-speeds', (speeds) => {
            updateFanSpeeds(leftFanSpeedChart, rightFanSpeedChart,
                speeds.payload.left_fan_speed, speeds.payload.right_fan_speed,
                speeds.payload.left_temp, speeds.payload.right_temp);
        });
    }

    await window.__TAURI__.core.invoke('get_fan_speeds');
    await listen_to_greet();
    console.log('📡 实时数据监听已启动');

    // 启动/停止按钮
    startStopButton.addEventListener('click', () => {
        isRunning = !isRunning;
        const fanDot = document.getElementById('fanStatusDot');
        const fanText = document.getElementById('fanStatusText');
        const btnText = document.getElementById('btnText');

        if (isRunning) {
            btnText.textContent = '停止控制';
            startStopButton.classList.add('running');

            // 更新状态指示器
            fanDot.className = 'status-dot active';
            fanText.textContent = '运行中';

            // 获取数据并传递给 Rust
            const fanData = getFanCurveData();
            window.__TAURI__.core.invoke('start_fan_control', {fanData});
            console.log('✅ 风扇控制已启动');
        } else {
            btnText.textContent = '启动控制';
            startStopButton.classList.remove('running');

            // 更新状态指示器
            fanDot.className = 'status-dot inactive';
            fanText.textContent = '待机';

            // 停止风扇控制
            window.__TAURI__.core.invoke('stop_fan_control');
            console.log('⏸️ 风扇控制已停止');
        }
    });

    // 加载配置按钮
    loadConfigButton.addEventListener('click', async () => {
        await loadConfigData();
    });

    // 保存配置按钮
    saveConfigButton.addEventListener('click', async () => {
        try {
            console.log('💾 正在保存风扇配置...');
            const fanData = getFanCurveData();
            await window.__TAURI__.core.invoke('save_fan_config', {fanData});

            // 更新状态指示器
            const configDot = document.getElementById('configStatusDot');
            const configText = document.getElementById('configStatusText');
            configDot.className = 'status-dot active';
            configText.textContent = '已保存';

            console.log('✅ 配置已保存');

            // 3秒后恢复为"已加载"状态
            setTimeout(() => {
                configText.textContent = '已加载';
            }, 3000);
        } catch (error) {
            console.error('❌ 配置保存失败:', error);

            const configDot = document.getElementById('configStatusDot');
            const configText = document.getElementById('configStatusText');
            configDot.className = 'status-dot warning';
            configText.textContent = '保存失败';

            alert('配置保存失败\n错误信息: ' + error);
        }
    });

    // 检查开机自启状态
    try {
        const autostartEnabled = await window.__TAURI__.core.invoke('plugin:autostart|is_enabled');
        console.log(`⚙️ 开机自启状态: ${autostartEnabled ? '已启用' : '未启用'}`);

        // 设置开关状态
        autostartCheckbox.checked = autostartEnabled;

        if(autostartEnabled) {
            console.log('🎯 检测到开机自启，自动加载配置并启动控制...');

            // 自动加载配置
            await loadConfigData();

            // 延迟1秒后自动启动风扇控制（确保配置加载完成）
            setTimeout(() => {
                startStopButton.click();
                console.log('✅ 开机自启完成');
            }, 1000);
        } else {
            console.log('💡 提示: 可在界面上启用开机自启功能');
        }
    } catch (error) {
        console.error('❌ 检查开机自启状态失败:', error);
    }

    // 开机自启动开关事件
    autostartCheckbox.addEventListener('change', async () => {
        try {
            if (autostartCheckbox.checked) {
                // 启用开机自启
                await window.__TAURI__.core.invoke('plugin:autostart|enable');
                console.log('✅ 已启用开机自启动');
                alert('开机自启动已启用\n程序将在系统启动时自动运行');
            } else {
                // 禁用开机自启
                await window.__TAURI__.core.invoke('plugin:autostart|disable');
                console.log('⏸️ 已禁用开机自启动');
                alert('开机自启动已禁用');
            }
        } catch (error) {
            console.error('❌ 设置开机自启失败:', error);
            // 恢复开关状态
            autostartCheckbox.checked = !autostartCheckbox.checked;
            alert('设置开机自启失败\n错误信息: ' + error);
        }
    });

    console.log('✨ NUCtool 初始化完成，系统就绪');
});

// 获取所有点信息并传递给 Rust
function getFanCurveData() {
    const leftFanData = leftFanCurve.data.labels.map((temp, index) => {
        return {temperature: temp, speed: leftFanCurve.data.datasets[0].data[index]};
    });

    const rightFanData = rightFanCurve.data.labels.map((temp, index) => {
        return {temperature: temp, speed: rightFanCurve.data.datasets[0].data[index]};
    });

    return {left_fan: leftFanData, right_fan: rightFanData};
}