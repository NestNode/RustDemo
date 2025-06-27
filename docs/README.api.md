# API

> [!warning]
> 
> 当前处于开发测试阶段，API尚不稳定，仅供参考

API 基本遵循 RESTful 设计

这里只有大概，具体见该文件夹路径下的 `api.md` / `api.apifox.json` (该文件由apifox导出，后者可通过导入apifox使用)

## REST

允许创建键值对的存储内容

- /rest
  - GET/POST
- /rest/{id}
  - GET/POST/PUT/PATCH/DELETE

## TODOS

同REST，只不过是为TODOS的应用场景，多做了一点工作。如 TODOS 的完成状态等

- /todos
  - GET/POST
- /todos/{id}
  - GET/POST/PUT/PATCH/DELETE

## NODE

允许创建节点对象（满足NODE特征 / 均为NODE的派生类）

- /node
  - GET/POST
- /node/{id}
  - GET/POST/PUT/PATCH/DELETE

## Other

一些杂七杂八的小工具，如心跳、状态查看等
