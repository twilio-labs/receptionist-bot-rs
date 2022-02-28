#/bin/sh
awslocal dynamodb create-table --cli-input-json  file:///docker-entrypoint-initaws.d/mock-ddb-create.json