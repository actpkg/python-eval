GET {{host}}/info
HTTP 200
[Asserts]
jsonpath "$.name" == "python-interpreter"
jsonpath "$.version" isString
