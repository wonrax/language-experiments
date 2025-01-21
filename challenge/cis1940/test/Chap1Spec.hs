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
  describe "hanoi tower" $ do
    it "hanoi 2" $ do
      hanoi 2 "a" "b" "c" `shouldBe` [("a", "c"), ("a", "b"), ("c", "b")]
    it "hanoi 3" $ do
      hanoi 3 "a" "b" "c" `shouldBe` [("a", "b"), ("a", "c"), ("b", "c"), ("a", "b"), ("c", "a"), ("c", "b"), ("a", "b")]
