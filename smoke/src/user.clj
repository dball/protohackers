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
      (.select selector)
      (let [keys (.selectedKeys selector)
            [key] (seq keys)]
        (.remove keys key)
        (cond

          (.isAcceptable key)
          (let [client (.accept serverSocketChannel)]
            (.configureBlocking client false)
            (.register client selector SelectionKey/OP_READ)
            (recur (assoc clients client {::reading true ::writes []})))

          (.isReadable key)
          (let [client (.channel key)
                buffer (ByteBuffer/allocate 64)
                read (.read client buffer)]
            (case read
              -1
              (do
                (.register client selector SelectionKey/OP_WRITE)
                (recur (assoc-in clients [client ::reading] false)))
              0
              (recur clients)
              (do
                (when-not (seq (get-in clients [client ::writes]))
                  (.register client selector (+ SelectionKey/OP_READ SelectionKey/OP_WRITE)))
                (recur (update-in clients [client ::writes] conj [read (.flip buffer)])))))

          (.isWritable key)
          (let [client (.channel key)
                {::keys [reading writes]} (get clients client)]
            (if (seq writes)
              (let [[[read buffer] & writes] writes
                    wrote (.write client buffer)]
                (when-not (= read wrote)
                  ;; TODO presumably we should reenqueue the partial write
                  (prn "mismatch" read wrote))
                (recur (assoc-in clients [client ::writes] writes)))
              (do
                (if reading
                  (.register client selector SelectionKey/OP_READ)
                  (.close client))
                (recur clients)))))))))