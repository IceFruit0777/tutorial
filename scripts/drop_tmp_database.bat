@REM 删除所有集成测试时生成的随机数据库
set PGHOST=localhost
set PGPORT=5432
set PGUSER=postgres
set PGPASSWORD=password
set BIN_PATH=E:\software\PostgreSQL\17\bin

@REM 查询所有非模板的数据库
for /f "delims=" %%i in (
    '%BIN_PATH%\psql -t -c "SELECT datname FROM pg_database WHERE datistemplate = false"'
) do (
    if "%%i" neq " postgres" (
        "%BIN_PATH%\dropdb" -h localhost -p 5432 -U postgres %%i
    )
)
