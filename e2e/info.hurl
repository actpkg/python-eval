GET {{baseurl}}/info
HTTP 200
[Asserts]
jsonpath "$['std:name']" == "python-eval"
jsonpath "$['std:version']" isString
