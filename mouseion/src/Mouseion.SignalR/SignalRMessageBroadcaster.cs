// SignalR message broadcaster service
using Microsoft.AspNetCore.SignalR;
using Serilog;

namespace Mouseion.SignalR;

public interface ISignalRMessageBroadcaster
{
    Task BroadcastMessage(SignalRMessage message);
}

public class SignalRMessageBroadcaster : ISignalRMessageBroadcaster
{
    private readonly IHubContext<MessageHub> _hubContext;
    private readonly ILogger _logger;

    public SignalRMessageBroadcaster(IHubContext<MessageHub> hubContext)
    {
        _hubContext = hubContext;
        _logger = Log.ForContext<SignalRMessageBroadcaster>();
    }

    public async Task BroadcastMessage(SignalRMessage message)
    {
        _logger.Debug("Broadcasting message: {MessageName}", message.Name);
        await _hubContext.Clients.All.SendAsync("receiveMessage", message);
    }
}
