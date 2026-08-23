module ListAppend
  ( append
  , appendNilRight
  , appendAssoc
  , sampleAppend
  , sampleAssoc
  , main
  ) where

append :: [a] -> [a] -> [a]
append [] ys = ys
append (x : xs) ys = x : append xs ys

appendNilRight :: [a] -> [a]
appendNilRight xs = append xs []

appendAssoc :: [a] -> [a] -> [a] -> [a]
appendAssoc xs ys zs = append (append xs ys) zs

sampleAppend :: [Int]
sampleAppend = append [1, 2, 3] [4, 5]

sampleAssoc :: [Int]
sampleAssoc = appendAssoc [1] [2, 3] [4]

main :: IO ()
main = do
  print sampleAppend
  print sampleAssoc
