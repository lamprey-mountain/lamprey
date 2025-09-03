import { createContext, ParentProps, useContext } from "solid-js";
import { createStore, SetStoreFunction } from "solid-js/store";
import { createVoiceClient } from "./rtc";

type VoiceSettings = {
	muted: boolean;
	deafened: boolean;
	cameraHidden: boolean;
	rtc: ReturnType<typeof createVoiceClient>,
};

const Voice = createContext<
	[VoiceSettings, SetStoreFunction<VoiceSettings>]
>();

export const useVoice = () => useContext(Voice)!;

export const VoiceProvider = (props: ParentProps) => {
	const rtc = createVoiceClient();

  // TODO: copy Voice.tsx stuff here
	const ctl = {
	  async toggleMicrophone() { },
	  async toggleCamera() { },
	  async stopScreenshare() { },
	  async startScreenshare() { },
	  async disconnect() { }, // do a FULL conn.close here
	  async connect(threadId: string) { },
	};

	const [state, update] = createStore({
		muted: true,
		deafened: false,
		cameraHidden: true,
		rtc,
	});

	return (
		<Voice.Provider value={[state, update]}>
			{props.children}
		</Voice.Provider>
	);
};
