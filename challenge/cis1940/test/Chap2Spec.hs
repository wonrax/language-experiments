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
