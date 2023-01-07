(ns user
  (:require [clojure.core.async :as async])
  (:import [java.nio.channels SelectionKey Selector ServerSocketChannel]
           [java.nio ByteBuffer]
           [java.net InetSocketAddress]))

(defn echo []
  (let [selector (Selector/open)
        serverSocketChannel (ServerSocketChannel/open)
        address (InetSocketAddress. "localhost" 9000)]
    (.configureBlocking serverSocketChannel false)
    (.bind (.socket serverSocketChannel) address)
    (.register serverSocketChannel selector SelectionKey/OP_ACCEPT nil)
    ))