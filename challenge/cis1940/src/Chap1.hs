module Chap1 where

-- toDigits :: Integer -> [Integer]
-- toDigits n
--   | n <= 0 = []
--   | otherwise = toDigits (n `div` 10) ++ [n `mod` 10]
--
-- doubleEveryOther :: [Integer] -> [Integer]
-- doubleEveryOther l = reverse $ zipWith (*) (cycle [1, 2]) (reverse l)
--
-- sumDigits :: [Integer] -> Integer
-- sumDigits = sum . concatMap toDigits
--
-- validateCreditCardNumber :: Integer -> Bool
-- validateCreditCardNumber x = (mod . sumDigits . doubleEveryOther . toDigits) x 10 == 0

validateCreditCardNumber :: Integer -> Bool
validateCreditCardNumber n
    | n <= 0 = False
    | otherwise =
        let digits = go n []
            doubled = zipWith (*) (cycle [1, 2]) (reverse digits)
            summed = sum [if x > 9 then x `div` 10 + x `mod` 10 else x | x <- doubled]
         in summed `mod` 10 == 0
  where
    go 0 acc = acc
    go x acc = go (x `div` 10) ((x `mod` 10) : acc)

type Peg = String
hanoi :: Integer -> Peg -> Peg -> Peg -> [(Peg, Peg)]
hanoi 0 _ _ _ = []
hanoi 1 a b _ = [(a, b)]
hanoi n source target aux = hanoi (n - 1) source aux target ++ [(source, target)] ++ hanoi (n - 1) aux target source
