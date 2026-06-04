import LeaderInviteCard from "../../features/squads/components/LeaderInviteCard";
import MemberActionsCard from "../../features/squads/components/MemberActionsCard";
import NoSquadState from "../../features/squads/components/NoSquadState";
import SquadInfoCard from "../../features/squads/components/SquadInfoCard";
import SquadMembersCard from "../../features/squads/components/SquadMembersCard";
import { resolveSquadViewMode } from "../../features/squads/lib/format";
import type { ProfileDashboardData } from "./types";

export default function ProfileSquadsTab({
  data,
  authToken,
  onChanged,
}: {
  data: ProfileDashboardData;
  authToken: string | null;
  onChanged: () => Promise<void>;
}) {
  if (!authToken) {
    return null;
  }

  const displayMode = resolveSquadViewMode(data);

  if (displayMode === "no_squad") {
    return (
      <div className="profile-squad-stage">
        <NoSquadState authToken={authToken} data={data} onChanged={onChanged} />
      </div>
    );
  }

  if (!data.squad) {
    return null;
  }

  if (displayMode === "leader") {
    return (
      <div className="profile-squad-stage">
        <div className="profile-split">
          <div className="profile-stack">
            <SquadInfoCard data={data} isLeader authToken={authToken} onChanged={onChanged} />
            <LeaderInviteCard authToken={authToken} data={data} onChanged={onChanged} />
          </div>
          <SquadMembersCard data={data} isLeader authToken={authToken} onChanged={onChanged} />
        </div>
      </div>
    );
  }

  return (
    <div className="profile-squad-stage">
      <div className="profile-split">
        <div className="profile-stack">
          <SquadInfoCard data={data} isLeader={false} authToken={authToken} onChanged={onChanged} />
          <MemberActionsCard authToken={authToken} squadId={data.squad.id} onChanged={onChanged} />
        </div>
        <SquadMembersCard
          data={data}
          isLeader={false}
          authToken={authToken}
          onChanged={onChanged}
        />
      </div>
    </div>
  );
}
