module Chap1Spec where

import Chap1
import Test.Hspec

spec :: Spec
spec = do
  describe "validateCreditCardNumber" $ do
    it "should return True for 4012888888881881" $ do
      validateCreditCardNumber 4012888888881881 `shouldBe` True
    it "should return False for 4012888888881882" $ do
      validateCreditCardNumber 4012888888881882 `shouldBe` False
