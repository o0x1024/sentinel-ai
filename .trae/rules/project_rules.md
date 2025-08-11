1.前端使用daisyui,风格应该简洁、舒适、美观
2.前后端交互使用json格式
3.前端的确定、提示、警告、错误等对话框应该使用daisyui的Modal组件
4.低交互的提示改成daisyui的Toast组件，进行弹出式提示
5.本项目是一个单机桌面部署架构，不需要集群部署，通过MCP进行扩展功能
6.中间和过程数据均使用数据库进行存取
7.项目技术栈 rust+tauri v2 + daisyui +vue3
8.cargo run 改成cargo check命令用于测试