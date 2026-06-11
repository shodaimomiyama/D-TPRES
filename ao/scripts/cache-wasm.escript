#!/usr/bin/env escript
%%! -sname formix_cache_client -setcookie hyperbeam_sandbox

%% cache-wasm.escript — Cache a WASM image on a running local HyperBEAM node.
%%
%% ~cache@1.0/write over HTTP only stores raw binaries (at `data/<hash>`),
%% which dev_wasm:init cannot resolve as an image. dev_wasm:cache_wasm_image
%% writes a proper message (`#{body => Bin}`) whose ID works as a process
%% `image` reference, so we call it over Erlang RPC instead.
%%
%% Usage: HB_NODE_SNAME=hb_sandbox escript cache-wasm.escript <abs-path-to-wasm>
%% Prints the 43-char image ID on stdout.

main([WasmPath]) ->
    Sname = os:getenv("HB_NODE_SNAME", "hb_sandbox"),
    {ok, Hostname} = inet:gethostname(),
    NodeName = list_to_atom(Sname ++ "@" ++ Hostname),
    case net_adm:ping(NodeName) of
        pong ->
            Result = rpc:call(NodeName, dev_wasm, cache_wasm_image,
                              [list_to_binary(WasmPath)]),
            case Result of
                #{<<"image">> := ImageID} ->
                    io:format("~s~n", [ImageID]);
                Other ->
                    io:format(standard_error, "Unexpected result: ~p~n", [Other]),
                    halt(1)
            end;
        pang ->
            io:format(standard_error,
                      "Cannot connect to ~s (is the HyperBEAM node running?)~n",
                      [NodeName]),
            halt(1)
    end;
main(_) ->
    io:format(standard_error,
              "Usage: escript cache-wasm.escript <abs-path-to-wasm>~n", []),
    halt(1).
