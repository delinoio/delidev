import { useState, useRef, useEffect } from "react";
import { Button } from "../components/ui/button";
import { Input } from "../components/ui/input";
import { Card } from "../components/ui/card";
import { Send, Mic } from "lucide-react";
import { cn } from "../lib/utils";

interface Message {
  id: string;
  role: "user" | "assistant";
  content: string;
  timestamp: Date;
}

export function Chat() {
  const [messages, setMessages] = useState<Message[]>([]);
  const [input, setInput] = useState("");
  const [isLoading, setIsLoading] = useState(false);
  const messagesEndRef = useRef<HTMLDivElement>(null);

  const scrollToBottom = () => {
    messagesEndRef.current?.scrollIntoView({ behavior: "smooth" });
  };

  useEffect(() => {
    scrollToBottom();
  }, [messages]);

  const handleSubmit = async (e: React.FormEvent) => {
    e.preventDefault();
    if (!input.trim() || isLoading) return;

    const userMessage: Message = {
      id: crypto.randomUUID(),
      role: "user",
      content: input.trim(),
      timestamp: new Date(),
    };

    setMessages((prev) => [...prev, userMessage]);
    setInput("");
    setIsLoading(true);

    // TODO: Implement actual AI chat integration
    // For now, show a placeholder response
    setTimeout(() => {
      const assistantMessage: Message = {
        id: crypto.randomUUID(),
        role: "assistant",
        content:
          "Chat functionality is not yet implemented. This is a placeholder response.",
        timestamp: new Date(),
      };
      setMessages((prev) => [...prev, assistantMessage]);
      setIsLoading(false);
    }, 1000);
  };

  return (
    <div className="flex flex-col h-[calc(100vh-8rem)]">
      <div className="mb-4">
        <h1 className="text-2xl font-bold">Chat</h1>
        <p className="text-muted-foreground">
          Interact with AI to create tasks, review code, and manage your
          projects.
        </p>
      </div>

      {/* Messages Area */}
      <div className="flex-1 overflow-y-auto space-y-4 mb-4">
        {messages.length === 0 ? (
          <div className="flex items-center justify-center h-full text-muted-foreground">
            <p>Start a conversation by typing a message below.</p>
          </div>
        ) : (
          messages.map((message) => (
            <Card
              key={message.id}
              className={cn(
                "p-4 max-w-[80%]",
                message.role === "user" ? "ml-auto bg-primary/10" : "mr-auto"
              )}
            >
              <div className="text-xs text-muted-foreground mb-1">
                {message.role === "user" ? "You" : "Assistant"}
              </div>
              <div className="whitespace-pre-wrap">{message.content}</div>
            </Card>
          ))
        )}
        {isLoading && (
          <Card className="p-4 max-w-[80%] mr-auto">
            <div className="text-xs text-muted-foreground mb-1">Assistant</div>
            <div className="flex gap-1">
              <span className="animate-bounce">.</span>
              <span className="animate-bounce delay-100">.</span>
              <span className="animate-bounce delay-200">.</span>
            </div>
          </Card>
        )}
        <div ref={messagesEndRef} />
      </div>

      {/* Input Area */}
      <form onSubmit={handleSubmit} className="flex gap-2">
        <Input
          value={input}
          onChange={(e) => setInput(e.target.value)}
          placeholder="Type a message..."
          disabled={isLoading}
          className="flex-1"
        />
        <Button type="button" variant="outline" size="icon" disabled>
          <Mic className="h-4 w-4" />
        </Button>
        <Button type="submit" size="icon" disabled={!input.trim() || isLoading}>
          <Send className="h-4 w-4" />
        </Button>
      </form>
    </div>
  );
}
