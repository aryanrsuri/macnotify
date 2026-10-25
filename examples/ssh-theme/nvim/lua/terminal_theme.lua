-- Theme-specific Neovim integration; independent of the Macnotify listener.
local M = {}

function M.apply()
  local selected = vim.fn.resolve(vim.fn.expand("~/.config/terminal-style/current"))
  local background = selected:match("([^/]+)$") == "light" and "light" or "dark"
  if vim.o.background ~= background or vim.g.colors_name ~= "reference" then
    vim.o.background = background
    vim.cmd.colorscheme("reference")
  end
end

function M.setup()
  M.apply()
  vim.api.nvim_create_autocmd({ "FocusGained", "BufEnter" }, {
    group = vim.api.nvim_create_augroup("TerminalTheme", { clear = true }),
    callback = M.apply,
  })
end

return M
