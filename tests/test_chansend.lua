-- Verify that `jet start`, when spawned from neovim via jobstart() over
-- plain pipes (i.e. NOT a pty), receives every line written with chansend.
--
-- Regression test for the bug where the stdin interrupt-byte watcher in
-- crates/cli/src/repl.rs was reading from STDIN_FILENO unconditionally —
-- in pipe mode it raced the pipe-mode BufReader::read_line and silently
-- swallowed every byte that wasn't 0x03 (^C). Symptom: only the FIRST
-- chansend line reached the kernel; everything after was dropped.

local MiniTest = require('mini.test')

local T = MiniTest.new_set()

local function repo_root()
  return vim.fn.fnamemodify(vim.fn.resolve(debug.getinfo(1).source:sub(2)), ':p:h:h')
end

local function ipykernel_available()
  return vim.fn.system({ 'python3', '-c', 'import ipykernel' }) ~= nil
    and vim.v.shell_error == 0
end

local function which(name)
  local out = vim.fn.system({ 'which', name })
  if vim.v.shell_error ~= 0 then return nil end
  out = (out:gsub('%s+$', ''))
  return out ~= '' and out or nil
end

local function ensure_python_kernelspec()
  local home = os.getenv('HOME') or ''
  local user = home .. '/Library/Jupyter/kernels/python3/kernel.json'
  if vim.fn.filereadable(user) == 1 then return user end

  local py = which('python3')
  if not py then return nil end
  local dir = vim.fn.tempname()
  vim.fn.mkdir(dir, 'p')
  local path = dir .. '/kernel.json'
  local spec = vim.json.encode({
    argv = { py, '-m', 'ipykernel_launcher', '-f', '{connection_file}' },
    display_name = 'Python (jet mini.test)',
    language = 'python',
    interrupt_mode = 'signal',
  })
  vim.fn.writefile({ spec }, path)
  return path
end

T['chansend delivers every line over pipe stdin'] = function()
  if not ipykernel_available() then
    MiniTest.skip('ipykernel not installed')
  end
  local kernel_json = ensure_python_kernelspec()
  if not kernel_json then
    MiniTest.skip('could not prepare python kernelspec')
  end

  local bin = repo_root() .. '/target/release/jet'
  if vim.fn.executable(bin) ~= 1 then
    MiniTest.skip('jet binary not built at ' .. bin .. '; run `cargo build --release -p jet_cli`')
  end

  local xdg = vim.fn.tempname()
  vim.fn.mkdir(xdg, 'p')

  -- Send three statements across separate chansend calls. The original bug
  -- only manifested on the second-and-later lines, after the in-flight
  -- execute kicked off the stdin watcher thread. The unique marker is
  -- printed *only* by the last line, so seeing it proves all three made
  -- it through.
  local marker = 'JETCHANSENDOK-' .. tostring(os.time()) .. '-' .. tostring(math.random(1e9))
  local output = {}
  local job_id = vim.fn.jobstart({ bin, 'start', kernel_json }, {
    env = { XDG_DATA_HOME = xdg },
    on_stdout = function(_, data, _)
      for _, line in ipairs(data) do
        table.insert(output, line)
      end
    end,
    on_stderr = function(_, data, _)
      for _, line in ipairs(data) do
        table.insert(output, line)
      end
    end,
  })
  MiniTest.expect.no_equality(job_id, 0)
  MiniTest.expect.no_equality(job_id, -1)

  -- Give jet a moment to launch the kernel and reach its prompt.
  vim.wait(3000)

  -- Three separate chansends: each list with a trailing "" terminates the
  -- line with a real NL, the exact pattern used in the bug report.
  vim.fn.chansend(job_id, { 'x = 1', '' })
  vim.fn.chansend(job_id, { 'x = x + 1', '' })
  vim.fn.chansend(job_id, { 'print("' .. marker .. ':" + str(x))', '' })

  local ok = vim.wait(15000, function()
    for _, line in ipairs(output) do
      if line:find(marker .. ':2', 1, true) then return true end
    end
    return false
  end, 100)

  vim.fn.jobstop(job_id)
  vim.fn.delete(xdg, 'rf')

  if not ok then
    error(
      'jet never produced the marker; chansend lines were swallowed.\n'
        .. 'expected to see "' .. marker .. ':2" in stdout.\n'
        .. 'got:\n  ' .. table.concat(output, '\n  ')
    )
  end
end

return T
