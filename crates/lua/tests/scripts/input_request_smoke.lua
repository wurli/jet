-- Smoke test: input_request round-trip.
-- Run `v = input(); print('GOT:'..v)`, drain until input_request shows up,
-- send a reply via provide_stdin, keep draining, assert GOT:hello arrives.

local jet = require('jet')

local spec = os.getenv('JET_TEST_KERNEL')
assert(spec and #spec > 0, 'JET_TEST_KERNEL env var must point to a kernel.json')

local kid = jet.connect(spec)
local poll = jet.execute_code(kid, "v = input('ASK> '); print('GOT:' + v)", {})

local saw_input_request = false
local saw_value = ''
local deadline = os.time() + 30

while os.time() < deadline do
  local r = poll()
  if r == nil then break end
  if r.status == 'busy' then
    if r.type == 'input_request' and not saw_input_request then
      saw_input_request = true
      jet.provide_stdin(kid, '', 'hello')
    elseif r.type == 'stream' and r.data and r.data.text then
      saw_value = saw_value .. r.data.text
    end
  elseif r.status == 'pending' then
    os.execute('sleep 0.01')
  end
end

jet.stop(kid)

assert(saw_input_request, 'never saw input_request frame')
assert(
  saw_value:find('GOT:hello'),
  string.format("expected 'GOT:hello' in stream output, got: %q", saw_value)
)
print('OK: input_request round-trip produced ' .. saw_value)
