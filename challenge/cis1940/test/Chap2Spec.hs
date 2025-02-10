module Chap2Spec where

import Chap2.Log
import Chap2.LogAnalysis
import Test.Hspec

spec :: Spec
spec = do
  describe "test parse logs" $ do
    it "should parse log" $ do
      result <- testParse parse 7 "challenge/cis1940/src/Chap2/sample.log"
      result
        `shouldBe` [ LogMessage Info 6 "Completed armadillo processing"
                   , LogMessage Info 1 "Nothing to report"
                   , LogMessage Info 4 "Everything normal"
                   , LogMessage Info 11 "Initiating self-destruct sequence"
                   , LogMessage (Error 70) 3 "Way too many pickles"
                   , LogMessage (Error 65) 8 "Bad pickle-flange interaction detected"
                   , LogMessage Warning 5 "Flange is due for a check-up"
                   ]
    it "should order log correctly" $
      do
        build
          [ LogMessage Info 6 "Completed armadillo processing"
          , LogMessage Info 1 "Nothing to report"
          , LogMessage Info 4 "Everything normal"
          , LogMessage Info 11 "Initiating self-destruct sequence"
          , LogMessage (Error 70) 3 "Way too many pickles"
          , LogMessage (Error 65) 8 "Bad pickle-flange interaction detected"
          , LogMessage Warning 5 "Flange is due for a check-up"
          ]
        `shouldBe` Node (Node (Node Leaf (LogMessage Info 1 "Nothing to report") Leaf) (LogMessage (Error 70) 3 "Way too many pickles") (Node Leaf (LogMessage Info 4 "Everything normal") Leaf)) (LogMessage Warning 5 "Flange is due for a check-up") (Node (Node Leaf (LogMessage Info 6 "Completed armadillo processing") Leaf) (LogMessage (Error 65) 8 "Bad pickle-flange interaction detected") (Node Leaf (LogMessage Info 11 "Initiating self-destruct sequence") Leaf))
    it "should inorder traverse correctly" $
      do
        inOrder (Node (Node (Node Leaf (LogMessage Info 1 "Nothing to report") Leaf) (LogMessage (Error 70) 3 "Way too many pickles") (Node Leaf (LogMessage Info 4 "Everything normal") Leaf)) (LogMessage Warning 5 "Flange is due for a check-up") (Node (Node Leaf (LogMessage Info 6 "Completed armadillo processing") Leaf) (LogMessage (Error 65) 8 "Bad pickle-flange interaction detected") (Node Leaf (LogMessage Info 11 "Initiating self-destruct sequence") Leaf)))
          `shouldBe` [LogMessage Info 1 "Nothing to report", LogMessage (Error 70) 3 "Way too many pickles", LogMessage Info 4 "Everything normal", LogMessage Warning 5 "Flange is due for a check-up", LogMessage Info 6 "Completed armadillo processing", LogMessage (Error 65) 8 "Bad pickle-flange interaction detected", LogMessage Info 11 "Initiating self-destruct sequence"]
    it "what went wrong" $ do
      logs <- testParse parse maxBound "challenge/cis1940/src/Chap2/error.log"
      print (whatWentWrong logs)

      whatWentWrong
        [ LogMessage Info 6 "Completed armadillo processing"
        , LogMessage Info 1 "Nothing to report"
        , LogMessage Info 4 "Everything normal"
        , LogMessage Info 11 "Initiating self-destruct sequence"
        , LogMessage (Error 65) 8 "Bad pickle-flange interaction detected"
        , LogMessage (Error 70) 3 "Way too many pickles"
        , LogMessage Warning 5 "Flange is due for a check-up"
        ]
        `shouldBe` ["Way too many pickles", "Bad pickle-flange interaction detected"]
