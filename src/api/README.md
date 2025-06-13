# README.api

## RESTful规范

符合RESTful规范

方法

- GET    | 查
- POST   | 增      | 重复策略: 覆盖 -> 报错 (409 Conflict)
- PUT    | 全部更新
- PATCH  | 部分更新 | 缺失策略: 报错
- DELETE | 全部删除

返回值

- 200 | StatusCode::OK         | 成功
- 201 | StatusCode::CREATED    | 成功创建
- 204 | StatusCode::NO_CONTENT | 删除成功，无需返回
- 404 | StatusCode::NOT_FOUND  | 找不到资源
- 409 | StatusCode::CONFLICT   | 重复

RESTful规范中，是否有规定POST增加时，重复项的策略是报错/副本/覆盖。PATCH部分更新时，缺失项的策略是报错/添加
