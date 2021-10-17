if !exists('s:serenadejobid')
	let s:serenadejobid = 0
endif

if !exists("g:os")
    if has("win64") || has("win32") || has("win16")
        let g:os = "Windows"
    else
        let g:os = substitute(system('uname'), '\n', '', '')
    endif
endif

" Path to the binary
let s:scriptdir = resolve(expand('<sfile>:p:h') . '/..')
let s:bin = s:scriptdir . '/target/debug/neovim-serenade'

" RPC message constants
let s:SerenadeStop = 'serenade_stop'
let s:SerenadeStart = 'serenade_start'

" Entry point
function! s:init()
  call s:connect()
endfunction

" Get the Job ID and check for errors. If no errors, attach RPC handlers to
" the commands.
function! s:connect()
  let jobID = s:GetJobID()

  if 0 == jobID
    echoerr "serenade: cannot start rpc process"
  elseif -1 == jobID
    echoerr "serenade: rpc process is not executable"
  else
    let s:serenadejobid = jobID
    call s:AttachRPCHandlers(jobID)
  endif
endfunction

" Function reference in case of RPC start errors
function! s:OnStderr(id, data, event) dict
  echom 'stderr: ' . a:event . join(a:data, "\n") 
endfunction

" Start the RPC job and return the job  (channel) ID
function! s:GetJobID()
  if s:serenadejobid == 0
    let jobid = jobstart([s:bin], { 'rpc': v:true, 'on_stderr': function('s:OnStderr') })
    return jobid
  else
    return s:serenadejobid
  endif
endfunction

" Associate commands with their RPC invocations
function! s:AttachRPCHandlers(jobID)
  command! -nargs=0 SerenadeStart :call s:rpc(s:SerenadeStart)
  command! -nargs=0 SerenadeStop :call s:rpc(s:SerenadeStop)
endfunction

" Send an RPC message to the remote process.
function! s:rpc(rpcMessage)
	call rpcnotify(s:serenadejobid, a:rpcMessage)
endfunction

call s:init()
