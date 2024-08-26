# Azure DevOps Repository Sync

This simple script is focused on syncing a local repo to a remote main branch in azure devops automatically. It can be added as a task or added to the startup tasks as a shortcut in windows.


## Running the Script on Windows Startup

1. Task Scheduler:
  	
    Use Windows Task Scheduler to run your Rust script at startup.

    Steps:
  	1.	Open Task Scheduler from the Start Menu.
  	2.	Select “Create Basic Task” from the right panel.
  	3.	Name the task (e.g., “DevOps Sync Script”).
  	4.	Choose “When the computer starts” as the trigger.
  	5.	Select “Start a Program” as the action.
  	6.	Browse to the compiled Rust executable (.exe) and select it.
  	7.	Configure the task to run with highest privileges if needed.
2. Startup Folder:

   Place a shortcut to the Rust executable in the Windows Startup folder.
	
   Steps:
	  1. Press Win + R and type shell:startup to open the Startup folder.
	  2. Create a shortcut to your Rust script’s executable in this folder.
  

For all of the above, make sure to have the config.toml file with the filled in values in the same directory as the executible file that runs.
