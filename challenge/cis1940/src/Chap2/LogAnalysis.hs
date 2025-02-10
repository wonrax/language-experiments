{-# LANGUAGE LambdaCase #-}

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

insert :: LogMessage -> MessageTree -> MessageTree
insert (Unknown _) tree = tree
insert log1 Leaf = Node Leaf log1 Leaf
insert log1 (Node left targetLog right)
  | extractTimestamp targetLog < extractTimestamp log1 = Node left targetLog (insert log1 right)
  | otherwise = Node (insert log1 left) targetLog right
 where
  extractTimestamp (LogMessage _ ts _) = ts
  extractTimestamp (Unknown _) = error "Cannot extract timestamp from Unknown log message"

build :: [LogMessage] -> MessageTree
build = foldr insert Leaf

inOrder :: MessageTree -> [LogMessage]
inOrder Leaf = []
inOrder (Node left dat right) = inOrder left ++ [dat] ++ inOrder right

whatWentWrong :: [LogMessage] -> [String]
whatWentWrong l =
  map
    ( \case
        LogMessage _ _ message -> message
        _ -> error "unreachable"
    )
    ( filter
        ( \case
            LogMessage (Error svr) _ _ -> svr >= 50
            _ -> False
        )
        (inOrder (build l))
    )
