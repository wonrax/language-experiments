module Chap2.LogAnalysis where

import Chap2.Log
import Text.Read

parseMessage :: String -> LogMessage
parseMessage m = case words m of
  "I" : ts : msg -> makeMessage Info ts msg
  "W" : ts : msg -> makeMessage Warning ts msg
  "E" : lvl : ts : msg -> case readMaybe lvl of
    Just severity -> makeMessage (Error severity) ts msg
    Nothing -> Unknown m
  _ -> Unknown m
 where
  makeMessage msgType ts msg = case readMaybe ts of
    Just timestamp -> LogMessage msgType timestamp (unwords msg)
    Nothing -> Unknown m

parse :: String -> [LogMessage]
parse = map parseMessage . lines
