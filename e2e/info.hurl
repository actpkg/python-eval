GET http://[::1]:{{port}}/info
HTTP 200
[Asserts]
jsonpath "$['std:name']" == "python-interpreter"
jsonpath "$['std:version']" isString
