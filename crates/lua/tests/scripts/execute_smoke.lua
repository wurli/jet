-- Smoke test: start a Python kernel, run print(1+1), drain frames until idle,
-- assert "2" appears in stream output. Exits 0 on success, nonzero on failure.

local jet = require('jet')

local spec = os.getenv('JET_TEST_KERNEL')
assert(spec and #spec > 0, 'JET_TEST_KERNEL env var must point to a kernel.json')

local kid, info = jet.connect(spec)
assert(type(kid) == 'string' and #kid > 0, 'expected session id from connect')
assert(type(info) == 'table', 'expected kernel info table')

local poll = jet.execute_code(kid, 'print(1+1)', {})
local saw = ''
local deadline = os.time() + 30
while os.time() < deadline do
  local r = poll()
  if r == nil then break end
  if r.status == 'busy' and r.type == 'stream' and r.data and r.data.text then
    saw = saw .. r.data.text
  end
  -- Tiny pause so we don't hammer try_recv. luajit's os.execute is the
  -- portable "sleep a few ms" hammer; for tests this is fine.
  if r.status == 'pending' then
    os.execute('sleep 0.01')
  end
end

jet.shutdown_kernel(kid)

assert(
  saw:find('2'),
  string.format("expected '2' in stream output, got: %q", saw)
)
print('OK: execute_code round-trip produced ' .. saw)
