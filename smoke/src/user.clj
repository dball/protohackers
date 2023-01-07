(ns user
  (:import [java.nio.channels SelectionKey Selector ServerSocketChannel]
           [java.nio ByteBuffer]
           [java.net InetSocketAddress]))

(defn -main []
  (let [selector (Selector/open)
        serverSocketChannel (ServerSocketChannel/open)
        address (InetSocketAddress. "localhost" 9000)]
    (.configureBlocking serverSocketChannel false)
    (.bind serverSocketChannel address)
    (.register serverSocketChannel selector SelectionKey/OP_ACCEPT nil)
    (loop []
      (.select selector)
      (let [keys (.selectedKeys selector)]
        (doseq [key keys]
          (cond
            (.isAcceptable key)
            (let [client (.accept serverSocketChannel)]
              (.configureBlocking client false)
              (.register client selector SelectionKey/OP_READ))
            (.isReadable key)
            (let [client (.channel key)
                  buffer (ByteBuffer/allocate 64)
                  read (.read client buffer)]
              (when (pos? read)
                (.flip buffer)
                ;; TODO non-blocking write
                (let [written (.write client buffer)]
                  (when (not= written read)
                    (throw (ex-data {:written written :read read}))))
                ))))
        (.clear keys))
      (recur))))