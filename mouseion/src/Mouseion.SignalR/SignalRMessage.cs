// SignalR message DTO
namespace Mouseion.SignalR;

public class SignalRMessage
{
    public string Name { get; set; } = string.Empty;
    public object? Body { get; set; }
}
