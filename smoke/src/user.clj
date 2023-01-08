(ns user
  (:gen-class)
  (:import [java.nio.channels SelectionKey Selector ServerSocketChannel]
           [java.nio ByteBuffer]
           [java.net InetSocketAddress]))

(defn -main []
  (let [selector (Selector/open)
        serverSocketChannel (ServerSocketChannel/open)
        address (InetSocketAddress. 9000)
        runtime (Runtime/getRuntime)]
    (.addShutdownHook
     runtime
     (Thread. (fn []
                (.close serverSocketChannel)
                (shutdown-agents)
                (.halt runtime 0))))
    (.configureBlocking serverSocketChannel false)
    (.bind serverSocketChannel address)
    (.register serverSocketChannel selector SelectionKey/OP_ACCEPT nil)
    (loop [clients {}]
      (prn "selecting")
      (.select selector)
      (let [keys (.selectedKeys selector)
            [key] (seq keys)]
        (prn "select" (count keys) (hash key))
        (.remove keys key)
        (cond

          (.isAcceptable key)
          (let [client (.accept serverSocketChannel)]
            (prn "accept" (hash client))
            (.configureBlocking client false)
            (.register client selector SelectionKey/OP_READ)
            (recur (assoc clients client {::reading true ::writes []})))

          (.isReadable key)
          (let [client (.channel key)
                buffer (ByteBuffer/allocate 8)
                read (.read client buffer)]
            (prn "read" read (hash client))
            (case read
              -1
              (do
                (prn "eof" (hash client))
                (.register client selector SelectionKey/OP_WRITE)
                (recur (assoc-in clients [client ::reading] false)))
              0
              (do
                (prn "empty" (hash client))
                (recur clients))
              (do
                (when-not (seq (get-in clients [client ::writes]))
                  (prn "register read-write" (hash client))
                  (.register client selector (+ SelectionKey/OP_READ SelectionKey/OP_WRITE)))
                (prn "queue buffer" (hash client) buffer (count (get-in clients [client ::writes])))
                (recur (update-in clients [client ::writes] conj [read (.flip buffer)])))))

          (.isWritable key)
          (let [client (.channel key)
                {::keys [reading writes]} (get clients client)]
            (prn "write" (hash client))
            (if (seq writes)
              (let [[[read buffer] & writes] writes
                    wrote (.write client buffer)]
                (prn "wrote" (hash client) buffer read wrote)
                (when-not (= read wrote)
                  ;; TODO presumably we should reenqueue the partial write
                  (prn "mismatch" (hash client) read wrote))
                (recur (assoc-in clients [client ::writes] writes)))
              (do
                (if reading
                  (do
                    (prn "register read" (hash client))
                    (.register client selector SelectionKey/OP_READ))
                  (do
                    (prn "close" (hash client))
                    (.close client)))
                (recur clients)))))))))