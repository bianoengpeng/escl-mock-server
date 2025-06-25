@echo off
chcp 65001 >nul
echo ==========================================
echo       eSCL Mock Scanner 一体化脚本
echo ==========================================
echo.

echo 请选择操作:
echo 1^) 快速启动 ^(推荐 - 你常用的配置^)
echo 2^) Windows 11 兼容模式
echo 3^) 网络诊断
echo 4^) 自定义配置
echo 5^) 退出
echo.
set /p choice="请输入选择 (1-5): "

if "%choice%"=="1" (
    echo.
    echo ==========================================
    echo 快速启动 - 使用你的常用配置
    echo ==========================================
    echo 命令: cargo run -- -a 0.0.0.0 -p 8080 -i res/portrait-color.jpg
    echo.
    cargo run -- -a 0.0.0.0 -p 8080 -i res/portrait-color.jpg
) else if "%choice%"=="2" (
    echo.
    echo ==========================================
    echo Windows 11 兼容模式启动
    echo ==========================================
    echo 检查网络配置...
    ipconfig | findstr "IPv4"
    echo.
    echo 检查防火墙状态...
    netsh advfirewall show allprofiles state | findstr "State"
    echo.
    echo 启动服务器 ^(Windows 11 优化配置^)...
    echo 命令: cargo run -- -a 0.0.0.0 -p 8080 -i res/portrait-color.jpg -s "/eSCL"
    echo.
    cargo run -- -a 0.0.0.0 -p 8080 -i res/portrait-color.jpg -s "/eSCL"
) else if "%choice%"=="3" (
    echo.
    echo ==========================================
    echo 网络诊断工具
    echo ==========================================
    set /p SERVER_IP="输入服务器IP地址 ^(默认 192.168.44.128^): "
    if "%SERVER_IP%"=="" set SERVER_IP=192.168.44.128
    set /p SERVER_PORT="输入端口 ^(默认 8080^): "
    if "%SERVER_PORT%"=="" set SERVER_PORT=8080
    
    echo.
    echo 测试网络连通性...
    ping -n 2 %SERVER_IP%
    echo.
    echo 测试HTTP连接...
    curl -s -o nul -w "HTTP状态码: %%%%{http_code}\n" http://%SERVER_IP%:%SERVER_PORT%/ 2>nul || echo 连接失败
    echo.
    echo 测试eSCL端点...
    curl -s -o nul -w "ScannerCapabilities: %%%%{http_code}\n" http://%SERVER_IP%:%SERVER_PORT%/eSCL/ScannerCapabilities 2>nul || echo 端点不可用
    echo.
) else if "%choice%"=="4" (
    echo.
    echo ==========================================
    echo 自定义配置
    echo ==========================================
    set /p custom_addr="绑定地址 ^(默认 0.0.0.0^): "
    if "%custom_addr%"=="" set custom_addr=0.0.0.0
    set /p custom_port="端口 ^(默认 8000^): "
    if "%custom_port%"=="" set custom_port=8000
    set /p custom_image="图片文件 ^(默认 res/portrait-color.jpg^): "
    if "%custom_image%"=="" set custom_image=res/portrait-color.jpg
    set /p custom_scope="eSCL范围 ^(可选，如 /eSCL^): "
    
    echo.
    if "%custom_scope%"=="" (
        echo 启动命令: cargo run -- -a %custom_addr% -p %custom_port% -i %custom_image%
        cargo run -- -a %custom_addr% -p %custom_port% -i %custom_image%
    ) else (
        echo 启动命令: cargo run -- -a %custom_addr% -p %custom_port% -i %custom_image% -s "%custom_scope%"
        cargo run -- -a %custom_addr% -p %custom_port% -i %custom_image% -s "%custom_scope%"
    )
) else if "%choice%"=="5" (
    echo 退出...
    exit /b 0
) else (
    echo 无效选择，使用快速启动...
    cargo run -- -a 0.0.0.0 -p 8000 -i res/portrait-color.jpg
)

echo.
echo ==========================================
echo 服务器已停止，按任意键退出...
pause >nul 