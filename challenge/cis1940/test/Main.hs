module Main (main) where

import Chap1

main :: IO ()
main = do
  print (validateCreditCardNumber 4012888888881881)
  print (validateCreditCardNumber 4012888888881882)
