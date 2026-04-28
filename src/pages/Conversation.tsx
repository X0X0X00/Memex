import { useEffect, useState } from "react"
import { useParams, Link } from "react-router-dom"
import { api } from "@/lib/api"
import type { Message } from "@/types"
import { MessageBubble } from "@/components/MessageBubble"

export default function Conversation() {
  const { id } = useParams()
  const [msgs, setMsgs] = useState<Message[]>([])
  const [loading, setLoading] = useState(true)

  useEffect(() => {
    if (!id) return
    setLoading(true)
    api.getConversation(decodeURIComponent(id))
      .then(setMsgs)
      .finally(() => setLoading(false))
  }, [id])

  if (loading) return <div className="p-8 text-muted-foreground">Loading…</div>
  if (!msgs.length) return <div className="p-8 text-muted-foreground">No messages.</div>

  return (
    <div>
      <div className="px-6 py-3 border-b border-border sticky top-0 bg-background/85 backdrop-blur z-10">
        <Link to="/library" className="text-sm text-muted-foreground hover:text-foreground">
          ← Library
        </Link>
      </div>
      {msgs.map(m => <MessageBubble key={m.id} m={m} />)}
    </div>
  )
}
