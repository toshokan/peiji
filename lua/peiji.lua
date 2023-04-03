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
local function charge_bucket(keys, args)
  local is_blocked = redis.call('GET', keys[2])
  if is_blocked
  then
    redis.call('SETEX', keys[2], args[6], 1)
    return {true, false, 0}
  end

  local total = 0
  local in_range = redis.call('XRANGE', keys[1], args[1], '+')
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

  if total < tonumber(args[2])
  then
    redis.call('XADD', keys[1], args[3], "src", "peiji", "cost", args[4])
    local new_total = total + args[4]
    if new_total >= tonumber(args[2])
    then
      redis.call('SETEX', keys[2], args[5], 1)
      return {true, true, new_total}
    else
      return {false, true, new_total}
    end
  else
    return {false, false, total}
  end
end

redis.register_function('charge_bucket', charge_bucket)
