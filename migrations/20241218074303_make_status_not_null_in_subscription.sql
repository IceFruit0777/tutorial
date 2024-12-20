BEGIN;
    -- 为历史记录回填status
    UPDATE subscription SET status='confirmed' WHERE status IS NULL;
    -- 更改`status`字段属性不允许为空
    ALTER TABLE subscription ALTER COLUMN status SET NOT NULL;
COMMIT;
