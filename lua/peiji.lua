#!lua name=peiji

-- keys[1] = bucket name
-- keys[2] = blocked bucket name
--
-- args[1] = current window beginning timestamp
-- args[2] = window maximum quota
-- args[3] = new charge id timestamp
-- args[4] = new charge amount
-- args[5] = quick block timeout
-- args[6] = repeat block timeout
--
-- Returns:
-- { is_blocked, charge_success, current_count, block_seconds }
-- 
local function charge_bucket(keys, args)
  local bucket_name, blocked_bucket_name = unpack(keys)
  local window_begin_ts,
        window_max_quota,
        new_charge_id,
        new_charge_amt,
        quick_block_timeout,
        repeat_block_timeout = unpack(args)
        
  local is_blocked = redis.call('GET', blocked_bucket_name)
  if is_blocked
  then
    redis.call('SETEX', blocked_bucket_name, repeat_block_timeout, 1)
    return {true, false, 0, repeat_block_timeout}
  end

  local total = 0
  local in_range = redis.call('XRANGE', bucket_name, window_begin_ts, '+')
  for i,v in ipairs(in_range)
  do
    local fields = v[2]
    for i = 1,#fields,2
    do
      if fields[i] == 'cost'
      then
        total = total + fields[i+1]
      end
    end
  end

  local new_total = total + new_charge_amt
  local max_quota_num = tonumber(window_max_quota)
  
  if new_total <= max_quota_num
  then
    redis.call('XADD', bucket_name, new_charge_id, "src", "peiji", "cost", new_charge_amt)
    return {false, true, new_total, 0}
  else
    redis.call('SETEX', blocked_bucket_name, quick_block_timeout, 1)
    return {true, false, total, quick_block_timeout}
  end
end

redis.register_function('charge_bucket', charge_bucket)
